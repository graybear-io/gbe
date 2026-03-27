//! Writ handler — oracle's capability implementations.
//!
//! Uses the common WritDispatcher from gbe-nexus. Manages a set of
//! running jobs (OracleDrivers) keyed by job_id.

use std::collections::HashMap;
use std::sync::Arc;

use frame::{NodeIdentity, WritResponse};
use frame::writ::WritFuture;
use gbe_jobs_domain::{JobDefinition, JobId, OrgId};
use gbe_nexus::EventEmitter;
use gbe_nexus::writ;
use serde_json::json;
use tokio::sync::Mutex;

use crate::driver::OracleDriver;

/// Tracks a running job managed by the oracle.
struct RunningJob {
    driver: OracleDriver,
}

/// Oracle's capability handler — plugs into WritDispatcher.
pub struct OracleCapabilities {
    identity: NodeIdentity,
    emitter: Arc<EventEmitter>,
    jobs: Arc<Mutex<HashMap<String, RunningJob>>>,
}

impl OracleCapabilities {
    pub fn new(identity: NodeIdentity, emitter: Arc<EventEmitter>) -> Self {
        Self {
            identity,
            emitter,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl writ::CapabilityHandler for OracleCapabilities {
    fn handle_capability<'a>(&'a self, w: &'a frame::Writ) -> WritFuture<'a> {
        Box::pin(async move {
            match w.capability.as_str() {
                "create-job" => self.create_job(w).await,
                "cancel-job" => self.cancel_job(w).await,
                "job-status" => self.job_status(w).await,
                _ => writ::unsupported(w, &self.identity),
            }
        })
    }
}

impl OracleCapabilities {
    async fn create_job(&self, w: &frame::Writ) -> WritResponse {
        // Extract the definition param from the writ payload
        let params = match writ::parse_params(w) {
            Ok(p) => p,
            Err(e) => return writ::error(w, &self.identity, &e),
        };

        let definition_str = match params.get("definition") {
            Some(serde_json::Value::String(s)) => s.clone(),
            _ => return writ::error(w, &self.identity, "missing required param: definition"),
        };

        // Parse as JobDefinition (YAML or JSON)
        // Try JSON first, then YAML
        let def: JobDefinition = if let Ok(d) = serde_json::from_str(&definition_str) {
            d
        } else if let Ok(d) = serde_yaml::from_str(&definition_str) {
            d
        } else {
            return writ::error(
                w,
                &self.identity,
                &format!("invalid job definition (tried JSON and YAML): {definition_str}"),
            );
        };

        // Generate IDs
        let job_id_str = format!("job_{}", ulid::Ulid::new());
        let job_id = match JobId::new(&job_id_str) {
            Ok(id) => id,
            Err(e) => return writ::error(w, &self.identity, &format!("bad job id: {e}")),
        };

        let org_id_str = params
            .get("org_id")
            .and_then(|v| v.as_str())
            .unwrap_or("org_default");
        let org_id = match OrgId::new(org_id_str) {
            Ok(id) => id,
            Err(e) => return writ::error(w, &self.identity, &format!("bad org id: {e}")),
        };

        let job_name = def.name.clone();
        let task_count = def.tasks.len();

        // Create the OracleDriver
        let driver =
            match OracleDriver::new(def, job_id.clone(), org_id, Some(self.emitter.clone())) {
                Ok(d) => d,
                Err(e) => {
                    return writ::error(w, &self.identity, &format!("job creation failed: {e}"));
                }
            };

        // Start it (emits JobCreated event)
        driver.start().await;

        // Get initial ready tasks
        let ready: Vec<String> = driver
            .ready_tasks()
            .iter()
            .map(|t| t.name.clone())
            .collect();

        // Store it
        let mut jobs = self.jobs.lock().await;
        jobs.insert(job_id_str.clone(), RunningJob { driver });

        writ::ok(
            w,
            &self.identity,
            json!({
                "job_id": job_id_str,
                "name": job_name,
                "task_count": task_count,
                "ready_tasks": ready,
                "status": "created"
            }),
        )
    }

    async fn cancel_job(&self, w: &frame::Writ) -> WritResponse {
        let params = match writ::parse_params(w) {
            Ok(p) => p,
            Err(e) => return writ::error(w, &self.identity, &e),
        };

        let job_id = match params.get("job_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return writ::error(w, &self.identity, "missing required param: job_id"),
        };

        let reason = params
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("cancelled via writ");

        // Take the job out of the map so we can drop the lock before awaiting cancel.
        let job = {
            let mut jobs = self.jobs.lock().await;
            jobs.remove(job_id)
        };

        match job {
            Some(mut job) => {
                job.driver.cancel(reason).await;
                writ::ok(
                    w,
                    &self.identity,
                    json!({
                        "job_id": job_id,
                        "status": "cancelled",
                        "reason": reason
                    }),
                )
            }
            None => writ::error(w, &self.identity, &format!("job not found: {job_id}")),
        }
    }

    async fn job_status(&self, w: &frame::Writ) -> WritResponse {
        let params = match writ::parse_params(w) {
            Ok(p) => p,
            Err(e) => return writ::error(w, &self.identity, &e),
        };

        let job_id = match params.get("job_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return writ::error(w, &self.identity, "missing required param: job_id"),
        };

        let jobs = self.jobs.lock().await;
        match jobs.get(job_id) {
            Some(job) => {
                let status = if job.driver.is_complete() {
                    "complete"
                } else if job.driver.is_failed() {
                    "failed"
                } else {
                    "running"
                };

                let ready: Vec<String> = job
                    .driver
                    .ready_tasks()
                    .iter()
                    .map(|t| t.name.clone())
                    .collect();

                writ::ok(
                    w,
                    &self.identity,
                    json!({
                        "job_id": job_id,
                        "name": job.driver.definition().name,
                        "status": status,
                        "ready_tasks": ready,
                        "task_count": job.driver.definition().tasks.len(),
                    }),
                )
            }
            None => writ::error(w, &self.identity, &format!("job not found: {job_id}")),
        }
    }
}

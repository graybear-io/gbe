//! Imprint — the shape-match → cell-projection mechanism.
//!
//! An imprint declares: "given data with this shape, produce these cells."
//! This is the v0 trait — hand-written Rust implementations. Future:
//! sigil-compiled imprints that generate the same trait impls.
//!
//! The imprint is the compilation boundary. Everything before it is
//! expressive authoring (sigils, natural language). Everything after
//! it is mechanical execution (field extraction, cell assembly).

use crate::page::Page;

/// A matter compiler imprint. Matches against structured data and
/// produces a page of cells.
///
/// Imprints are registered with a matter compiler in specificity order.
/// The most specific matching imprint wins.
pub trait Imprint: Send + Sync {
    /// A human-readable name for this imprint (for debugging/discovery).
    fn name(&self) -> &str;

    /// Check whether this imprint can handle the given data.
    ///
    /// The data is the raw structured payload from the bus, plus the
    /// subject it arrived on. Shape matching: return true if the data
    /// has the fields this imprint needs.
    fn matches(&self, subject: &str, data: &serde_json::Value) -> bool;

    /// Specificity score. Higher = more specific. When multiple imprints
    /// match, the highest specificity wins.
    ///
    /// Convention: default imprints return 0, domain-specific return 10,
    /// fully-typed return 20. Custom imprints can use any value.
    fn specificity(&self) -> u32;

    /// Produce a page from the matched data.
    ///
    /// Called only when `matches` returned true. The implementation
    /// extracts fields, wraps them in typed content, assigns roles
    /// and links, and bundles into a page.
    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page;
}

/// A registry of imprints, ordered by specificity.
///
/// When data arrives, the registry finds the most specific matching
/// imprint and delegates compilation to it.
pub struct ImprintRegistry {
    imprints: Vec<Box<dyn Imprint>>,
}

impl ImprintRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            imprints: Vec::new(),
        }
    }

    /// Register an imprint. The registry maintains specificity order.
    pub fn register(&mut self, imprint: Box<dyn Imprint>) {
        self.imprints.push(imprint);
        // Sort descending by specificity so we can short-circuit on first match.
        self.imprints
            .sort_by(|a, b| b.specificity().cmp(&a.specificity()));
    }

    /// Find the most specific imprint that matches and compile the data.
    ///
    /// Returns `None` if no imprint matches.
    pub fn compile(&self, subject: &str, data: &serde_json::Value) -> Option<Page> {
        self.imprints
            .iter()
            .find(|imp| imp.matches(subject, data))
            .map(|imp| {
                let mut page = imp.compile(subject, data);
                page.index.compiled_by = Some(imp.name().to_string());
                page
            })
    }

    /// Compile with a full pipeline trace.
    ///
    /// Returns the compiled page and a trace showing what happened:
    /// which imprints were tested, which matched, which won.
    pub fn compile_traced(&self, subject: &str, data: &serde_json::Value) -> CompileTrace {
        let mut tested = Vec::new();

        for imp in &self.imprints {
            let matched = imp.matches(subject, data);
            tested.push(ImprintTrace {
                name: imp.name().to_string(),
                specificity: imp.specificity(),
                matched,
            });

            if matched {
                let mut page = imp.compile(subject, data);
                page.index.compiled_by = Some(imp.name().to_string());
                return CompileTrace {
                    subject: subject.to_string(),
                    raw: data.clone(),
                    tested,
                    page: Some(page),
                };
            }
        }

        CompileTrace {
            subject: subject.to_string(),
            raw: data.clone(),
            tested,
            page: None,
        }
    }

    /// How many imprints are registered.
    pub fn len(&self) -> usize {
        self.imprints.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.imprints.is_empty()
    }
}

impl Default for ImprintRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The full pipeline trace from a compile operation.
#[derive(Debug, Clone)]
pub struct CompileTrace {
    /// The bus subject that triggered compilation.
    pub subject: String,
    /// The raw data as it arrived from the bus.
    pub raw: serde_json::Value,
    /// Each imprint that was tested, in specificity order.
    pub tested: Vec<ImprintTrace>,
    /// The compiled page, if any imprint matched.
    pub page: Option<Page>,
}

/// One imprint's result during the match phase.
#[derive(Debug, Clone)]
pub struct ImprintTrace {
    /// Imprint name.
    pub name: String,
    /// Imprint specificity score.
    pub specificity: u32,
    /// Whether this imprint matched the data.
    pub matched: bool,
}

impl std::fmt::Display for CompileTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "═══ Pipeline Trace ═══")?;
        writeln!(f)?;

        // Stage 1: raw input.
        writeln!(f, "── Stage 1: Bus Event ──")?;
        writeln!(f, "subject: {}", self.subject)?;
        let raw_pretty = serde_json::to_string_pretty(&self.raw).unwrap_or_default();
        for line in raw_pretty.lines() {
            writeln!(f, "  {line}")?;
        }
        writeln!(f)?;

        // Stage 2: imprint matching.
        writeln!(f, "── Stage 2: Imprint Matching ──")?;
        for trace in &self.tested {
            let marker = if trace.matched { "→" } else { " " };
            let status = if trace.matched { "MATCHED" } else { "skipped" };
            writeln!(
                f,
                "  {marker} {} (specificity {}) — {}",
                trace.name, trace.specificity, status
            )?;
        }
        writeln!(f)?;

        // Stage 3: compiled page.
        match &self.page {
            Some(page) => {
                writeln!(f, "── Stage 3: Compiled Page ──")?;
                writeln!(f, "type: {}", page.index.content_type)?;
                if let Some(source) = &page.index.source {
                    writeln!(f, "source: {source}")?;
                }
                if let Some(compiled_by) = &page.index.compiled_by {
                    writeln!(f, "imprint: {compiled_by}")?;
                }
                writeln!(f, "cells: {}", page.cells.len())?;
                writeln!(f)?;

                for (i, cell) in page.cells.iter().enumerate() {
                    let role = format!("{:?}", cell.role).to_uppercase();
                    writeln!(f, "  [{i}] {role} (priority: {})", cell.priority)?;

                    // Content.
                    let content = crate::content_display::render_compact(&cell.content);
                    writeln!(f, "      content: {content}")?;

                    // Links.
                    if !cell.links.is_empty() {
                        let links: Vec<String> = cell
                            .links
                            .iter()
                            .map(|link| {
                                let kind = format!("{:?}", link.kind);
                                // Find the index of the target cell.
                                let target_idx = page
                                    .cells
                                    .iter()
                                    .position(|c| c.id == link.target)
                                    .map(|i| format!("[{i}]"))
                                    .unwrap_or_else(|| "?".to_string());
                                format!("{kind} → {target_idx}")
                            })
                            .collect();
                        writeln!(f, "      links: {}", links.join(", "))?;
                    }
                }
            }
            None => {
                writeln!(f, "── Stage 3: No Match ──")?;
                writeln!(f, "No imprint matched this data.")?;
            }
        }

        Ok(())
    }
}

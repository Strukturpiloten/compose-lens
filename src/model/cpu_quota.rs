//! Raw-preserving service CPU-quota values.

/// A service `cpu_quota` scalar category with exact authored spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CpuQuota {
    /// A YAML numeric scalar retained without conversion or normalization.
    YamlNumber(String),
    /// A YAML string scalar retained without numeric coercion.
    String(String),
}

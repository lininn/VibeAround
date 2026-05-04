//! Provider plugins — model-provider catalog metadata.
//!
//! Provider plugins are static metadata plugins. They do not spawn a runtime;
//! instead their `plugin.json` carries a `providerCatalog` object that uses the
//! same schema as the built-in profile catalog.

use std::collections::HashMap;

use crate::profiles::catalog::ProviderCatalog;

use super::DiscoveredPlugin;

const PROVIDER_PLUGIN_KIND: &str = "provider";

/// All provider-kind plugins keyed by plugin id.
pub fn discover() -> HashMap<String, DiscoveredPlugin> {
    super::discover_plugins()
        .into_iter()
        .filter(|(_, plugin)| plugin.manifest.kind == PROVIDER_PLUGIN_KIND)
        .collect()
}

/// Catalog entries supplied by user/project provider plugins.
pub fn catalogs() -> Vec<ProviderCatalog> {
    let mut catalogs = discover()
        .values()
        .filter_map(|plugin| plugin.manifest.provider_catalog.clone())
        .collect::<Vec<_>>();
    catalogs.sort_by(|left, right| left.id.cmp(&right.id));
    catalogs
}

/// Look up a single provider plugin by id.
pub fn find(plugin_id: &str) -> Option<DiscoveredPlugin> {
    discover().remove(plugin_id)
}

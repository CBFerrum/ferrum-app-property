//! Maintenance requests: an urgent one takes its unit out of service until it is done, and the day
//! it was done is kept.

use crate::{obj, refresh_unit};
use async_trait::async_trait;
use ferrum_domain::util::{elevated, id, s};
use ferrum_kernel::{Doc, DocHook, HookCtx, Result};
use serde_json::json;

pub struct RequestHook;

#[async_trait]
impl DocHook for RequestHook {
    fn name(&self) -> &'static str {
        "property.request"
    }

    async fn after_save(&self, hc: &mut HookCtx<'_>, doc: &Doc, old: Option<&Doc>) -> Result<()> {
        let system = elevated(hc.ctx);
        if s(doc, "status") == Some("Done")
            && old.is_none_or(|o| s(o, "status") != Some("Done"))
            && let Some(request) = id(doc, "id")
        {
            hc.kernel.set_values_tx(&system, hc.tx, "RepairRequest", request, obj(json!({"done_on": hc.ctx.today().to_string()}))).await?;
        }
        // The unit it is on now, and the one it was moved from.
        let mut units = vec![id(doc, "unit")];
        if let Some(o) = old {
            units.push(id(o, "unit"));
        }
        units.dedup();
        for unit in units.into_iter().flatten() {
            refresh_unit(hc.kernel, &system, hc.tx, unit).await?;
        }
        Ok(())
    }
}

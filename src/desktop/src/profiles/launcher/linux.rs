use super::common::LaunchPlan;

pub(super) fn spawn(_plan: LaunchPlan) -> anyhow::Result<()> {
    anyhow::bail!("Profile launch is only supported on macOS and Windows");
}

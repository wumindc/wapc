//! WAPC native desktop entrypoint.
//! @author codex

fn main() -> anyhow::Result<()> {
    wapc::desktop::run_desktop()
}

use version_gen::gen_clash_version;

fn main() -> anyhow::Result<()> {
    gen_clash_version(env!("CARGO_MANIFEST_DIR"))
}

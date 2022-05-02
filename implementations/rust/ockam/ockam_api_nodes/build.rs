const PROTOS: &[&str] = &["proto/api.nodes.proto"];

fn main() -> std::io::Result<()> {
    let mut c = prost_build::Config::new();
    c.btree_map(&["."]); // to support no_std
    c.compile_protos(PROTOS, &["proto/"])?;
    Ok(())
}

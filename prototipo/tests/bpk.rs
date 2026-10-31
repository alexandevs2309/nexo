use basilisco::cliente_remoto;
use std::io::Write;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
fn bpk_valido() {
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default();
        zip.start_file("BPK-MANIFEST.toml", opts).unwrap();
        zip.write_all(
            b"[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[[files]]\npath = \"lib.bsk\"\n",
        )
        .unwrap();
        zip.start_file("payload/lib.bsk", opts).unwrap();
        zip.write_all(b"1\n").unwrap();
        zip.finish().unwrap();
    }
    // cursor into buf? The cursor borrowed buf; need extract.
    // Rebuild simpler: use Vec directly.
    let mut buf2 = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf2));
        let opts = SimpleFileOptions::default();
        zip.start_file("BPK-MANIFEST.toml", opts).unwrap();
        zip.write_all(
            b"[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[[files]]\npath = \"lib.bsk\"\n",
        )
        .unwrap();
        zip.start_file("payload/lib.bsk", opts).unwrap();
        zip.write_all(b"1\n").unwrap();
        zip.finish().unwrap();
    }
    assert!(cliente_remoto::validar_bpk(&buf2).is_ok());
}

#[test]
fn bpk_sin_manifest() {
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default();
        zip.start_file("lib.bsk", opts).unwrap();
        zip.write_all(b"1\n").unwrap();
        zip.finish().unwrap();
    }
    assert!(cliente_remoto::validar_bpk(&buf).is_err());
}

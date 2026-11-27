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
            b"[format]\nname = \"BASILISCO_PACKAGE\"\nversion = \"1.0\"\n\n[package]\nnamespace = \"demo\"\nname = \"demo\"\nversion = \"0.1.0\"\ndescription = \"demo\"\nlicense = \"Apache-2.0\"\nauthors = [\"demo\"]\n\n[compatibility]\nbasilisco = \">=0.6.0\"\n\n[dependencies]\n\n[[files]]\npath = \"lib.bsk\"\nsize = 2\nsha256 = \"4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865\"\n",
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
            b"[format]\nname = \"BASILISCO_PACKAGE\"\nversion = \"1.0\"\n\n[package]\nnamespace = \"demo\"\nname = \"demo\"\nversion = \"0.1.0\"\ndescription = \"demo\"\nlicense = \"Apache-2.0\"\nauthors = [\"demo\"]\n\n[compatibility]\nbasilisco = \">=0.6.0\"\n\n[dependencies]\n\n[[files]]\npath = \"lib.bsk\"\nsize = 2\nsha256 = \"4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865\"\n",
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

#[test]
fn bpk_rechaza_path_traversal() {
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default();
        zip.start_file("BPK-MANIFEST.toml", opts).unwrap();
        zip.write_all(b"[format]\nname=\"BASILISCO_PACKAGE\"\nversion=\"1.0\"\n[package]\nnamespace=\"d\"\nname=\"d\"\nversion=\"0.1.0\"\n[compatibility]\nbasilisco=\">=0.0.0\"\n[dependencies]\n[[files]]\npath=\"lib.bsk\"\nsize=2\nsha256=\"4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865\"\n").unwrap();
        zip.start_file("payload/../evil.txt", opts).unwrap();
        zip.write_all(b"x\n").unwrap();
        zip.finish().unwrap();
    }
    assert!(cliente_remoto::validar_bpk(&buf).is_err());
    let dir = std::env::temp_dir().join(format!("bpk-trav-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(cliente_remoto::extraer_bpk(&buf, &dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

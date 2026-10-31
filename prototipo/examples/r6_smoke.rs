use basilisco::paquetes::{self};
use ed25519_dalek::SigningKey;
use std::io::{Read, Write};

fn main() {
    let base = std::path::Path::new("/tmp/opencode/smoke");
    let _ = std::fs::remove_dir_all(base);
    std::fs::create_dir_all(base).unwrap();

    // 1. login
    let login_body = serde_json::json!({"username":"admin","password":"adminpass123"});
    let resp = ureq::post("http://127.0.0.1:8099/api/v1/auth/token")
        .header("Content-Type", "application/json")
        .send(login_body.to_string())
        .unwrap();
    let txt = resp.into_body().read_to_string().unwrap();
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    let token = v["data"]["token"].as_str().unwrap().to_string();
    println!("token ok");

    // 2. create namespace
    let ns_body = serde_json::json!({"name":"demo"});
    let r = ureq::post("http://127.0.0.1:8099/api/v1/namespaces")
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send(ns_body.to_string());
    println!("ns create: {:?}", r.map(|x| x.status()));

    // 3. keys
    let keys_resp = ureq::get("http://127.0.0.1:8099/api/v1/keys")
        .call()
        .unwrap();
    let txt = keys_resp.into_body().read_to_string().unwrap();
    let kv: serde_json::Value = serde_json::from_str(&txt).unwrap();
    let key_id = kv["data"]["keys"][0]["key_id"]
        .as_str()
        .unwrap()
        .to_string();
    let pk_hex = kv["data"]["keys"][0]["public_key"]
        .as_str()
        .unwrap()
        .to_string();
    println!("key_id={} pk={}", key_id, &pk_hex[..16]);

    // signing key (publisher). Fixed seed for reproducibility.
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let vk = sk.verifying_key();
    let vk_hex: String = vk.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();

    // trust store: register publisher public key
    let mut almacen = paquetes::AlmacenClaves::default();
    almacen
        .agregar_seguro("registro-local", &key_id, vk.to_bytes().to_vec())
        .unwrap();
    almacen.guardar(base).unwrap();

    // 4. make BPK
    let bpk = make_bpk();
    let sha = paquetes::sha256_hex(&bpk);

    let meta = paquetes::MetadatosPaquete {
        namespace: "demo".to_string(),
        name: "demo".to_string(),
        version: "0.1.0".to_string(),
        sha256: sha.clone(),
        published_at: "2026-10-31T00:00:00Z".to_string(),
        registry_id: "registro-local".to_string(),
        key_id: key_id.clone(),
        nonce: "nonce-smoke-1".to_string(),
        expires_at: "2027-10-31T00:00:00Z".to_string(),
        firma: None,
        clave: Some(vk_hex.clone()),
    };
    let firma = paquetes::firmar_metadatos(&meta, &sk.to_bytes()).unwrap();
    let meta2 = paquetes::MetadatosPaquete {
        firma: Some(firma.clone()),
        ..meta
    };

    // server MetadatosPublicacion requires published_at field (added). Build JSON matching server model.
    let pub_meta = serde_json::json!({
        "namespace": meta2.namespace,
        "name": meta2.name,
        "version": meta2.version,
        "sha256": meta2.sha256,
        "registry_id": meta2.registry_id,
        "key_id": meta2.key_id,
        "nonce": meta2.nonce,
        "expires_at": meta2.expires_at,
        "published_at": meta2.published_at,
        "firma": meta2.firma.clone().unwrap(),
        "clave": meta2.clave.clone().unwrap(),
    });
    let b64 = base64_encode(&bpk);
    let pub_body = serde_json::json!({"meta": pub_meta, "artifact_b64": b64});

    let r = ureq::post("http://127.0.0.1:8099/api/v1/publish")
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send(pub_body.to_string());
    match r {
        Ok(resp) => println!(
            "publish: {} {}",
            resp.status(),
            resp.into_body().read_to_string().unwrap_or_default()
        ),
        Err(e) => {
            println!("publish err: {}", e);
            return;
        }
    }

    // 5. info version
    let info = ureq::get("http://127.0.0.1:8099/api/v1/packages/demo/demo/0.1.0")
        .call()
        .unwrap();
    let txt = info.into_body().read_to_string().unwrap();
    let iv: serde_json::Value = serde_json::from_str(&txt).unwrap();
    println!(
        "info data keys: {:?}",
        iv["data"]
            .as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
    );

    // 6. download
    let dl = ureq::get("http://127.0.0.1:8099/api/v1/packages/demo/demo/0.1.0/download")
        .call()
        .unwrap();
    let mut buf = Vec::new();
    dl.into_body().as_reader().read_to_end(&mut buf).unwrap();
    assert_eq!(paquetes::sha256_hex(&buf), sha);
    println!("download sha ok, size {}", buf.len());

    // 7. install via client
    let meta_url = "http://127.0.0.1:8099/api/v1/packages/demo/demo/0.1.0";
    let art_url = "http://127.0.0.1:8099/api/v1/packages/demo/demo/0.1.0/download";
    let res = paquetes::instalar_paquete_remoto_completo(base, meta_url, art_url);
    match res {
        Ok(r) => println!(
            "install ok: {} {} verified={}",
            r.nombre, r.version, r.verificado
        ),
        Err(e) => println!("install err: {}", e),
    }

    // 8. restart survival not tested here
    println!("SMOKE DONE");
}

fn make_bpk() -> Vec<u8> {
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;
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
        zip.write_all(b"lib demo\n").unwrap();
        zip.finish().unwrap();
    }
    buf
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

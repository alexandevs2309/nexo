use basilisco::paquetes;
use ed25519_dalek::SigningKey;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::thread;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn hacer_bpk() -> Vec<u8> {
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

#[test]
fn remoto_e2e_valido() {
    let dir = std::env::temp_dir().join(format!("basilisco_e2e_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    // Claves
    let sk = SigningKey::from_bytes(&[0u8; 32]);
    let vk = sk.verifying_key();
    let vk_bytes = vk.to_bytes().to_vec();
    let vk_hex: String = vk_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    // Almacén de confianza
    let mut almacen = paquetes::AlmacenClaves::default();
    almacen
        .agregar_seguro("registro1", "key1", vk_bytes.clone())
        .unwrap();
    almacen.guardar(&dir).unwrap();

    let bpk = hacer_bpk();
    let sha = paquetes::sha256_hex(&bpk);

    // Metadatos
    let meta = paquetes::MetadatosPaquete {
        namespace: "demo".to_string(),
        name: "demo".to_string(),
        version: "0.1.0".to_string(),
        sha256: sha.clone(),
        published_at: "2026-10-31T00:00:00Z".to_string(),
        registry_id: "registro1".to_string(),
        key_id: "key1".to_string(),
        nonce: "nonce-e2e-1".to_string(),
        expires_at: "2027-10-31T00:00:00Z".to_string(),
        firma: None,
        clave: Some(vk_hex.clone()),
    };
    let firma = paquetes::firmar_metadatos(&meta, &sk.to_bytes()).unwrap();
    let meta2 = paquetes::MetadatosPaquete {
        firma: Some(firma),
        clave: Some(vk_hex),
        ..meta
    };
    let meta_json = serde_json::to_string(&meta2).unwrap();

    // Servidor HTTP local
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let bpk_clone = bpk.clone();
    thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = stream.unwrap();
            let mut buf = [0; 1024];
            let _ = stream.read(&mut buf);
            let req = String::from_utf8_lossy(&buf);
            if req.contains("/meta") {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    meta_json.len(),
                    meta_json
                );
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
            } else {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bpk_clone.len()
                );
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
                let _ = std::io::Write::write_all(&mut stream, &bpk_clone);
            }
        }
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let meta_url = format!("http://{}/meta", addr);
    let art_url = format!("http://{}/art", addr);

    let res = paquetes::instalar_paquete_remoto_completo(&dir, &meta_url, &art_url).unwrap();
    assert_eq!(res.nombre, "demo");
    assert_eq!(res.version, "0.1.0");
    assert!(res.verificado);

    // Verificar lock
    let lock = paquetes::leer_bloqueo(&dir).unwrap().unwrap();
    assert!(!lock.is_empty());

    // Verificar caché
    let cache = dir
        .join(".basilisco")
        .join("cache")
        .join("demo")
        .join("0.1.0");
    assert!(cache.exists());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn remoto_e2e_firma_invalida() {
    let dir = std::env::temp_dir().join(format!("basilisco_e2e_f_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let sk = SigningKey::from_bytes(&[1u8; 32]);
    let vk = sk.verifying_key();
    let vk_bytes = vk.to_bytes().to_vec();
    let vk_hex: String = vk_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    let mut almacen = paquetes::AlmacenClaves::default();
    almacen
        .agregar_seguro("registro1", "key1", vk_bytes.clone())
        .unwrap();
    almacen.guardar(&dir).unwrap();

    let bpk = hacer_bpk();
    let sha = paquetes::sha256_hex(&bpk);

    let meta = paquetes::MetadatosPaquete {
        namespace: "demo".to_string(),
        name: "demo".to_string(),
        version: "0.1.0".to_string(),
        sha256: sha.clone(),
        published_at: "2026-10-31T00:00:00Z".to_string(),
        registry_id: "registro1".to_string(),
        key_id: "key1".to_string(),
        nonce: "nonce-f-1".to_string(),
        expires_at: "2027-10-31T00:00:00Z".to_string(),
        firma: Some("AAAA".to_string()),
        clave: Some(vk_hex),
    };
    let meta_json = serde_json::to_string(&meta).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let bpk_clone = bpk.clone();
    thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = stream.unwrap();
            let mut buf = [0; 1024];
            let _ = stream.read(&mut buf);
            let req = String::from_utf8_lossy(&buf);
            if req.contains("/meta") {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    meta_json.len(),
                    meta_json
                );
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
            } else {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bpk_clone.len()
                );
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
                let _ = std::io::Write::write_all(&mut stream, &bpk_clone);
            }
        }
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let meta_url = format!("http://{}/meta", addr);
    let art_url = format!("http://{}/art", addr);

    let res = paquetes::instalar_paquete_remoto_completo(&dir, &meta_url, &art_url);
    assert!(res.is_err());
    // Sin lock parcial
    assert!(paquetes::leer_bloqueo(&dir).unwrap().is_none());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn remoto_e2e_clave_desconocida() {
    let dir = std::env::temp_dir().join(format!("basilisco_e2e_c_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let sk = SigningKey::from_bytes(&[2u8; 32]);
    let vk = sk.verifying_key();
    let vk_bytes = vk.to_bytes().to_vec();
    let vk_hex: String = vk_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    // Sin clave confiable
    let almacen = paquetes::AlmacenClaves::default();
    almacen.guardar(&dir).unwrap();

    let bpk = hacer_bpk();
    let sha = paquetes::sha256_hex(&bpk);

    let mut meta = paquetes::MetadatosPaquete {
        namespace: "demo".to_string(),
        name: "demo".to_string(),
        version: "0.1.0".to_string(),
        sha256: sha.clone(),
        published_at: "2026-10-31T00:00:00Z".to_string(),
        registry_id: "registro1".to_string(),
        key_id: "key1".to_string(),
        nonce: "nonce-c-1".to_string(),
        expires_at: "2027-10-31T00:00:00Z".to_string(),
        firma: None,
        clave: Some(vk_hex.clone()),
    };
    let firma = paquetes::firmar_metadatos(&meta, &sk.to_bytes()).unwrap();
    meta.firma = Some(firma);
    meta.clave = Some(vk_hex);

    let meta_json = serde_json::to_string(&meta).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let bpk_clone = bpk.clone();
    thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = stream.unwrap();
            let mut buf = [0; 1024];
            let _ = stream.read(&mut buf);
            let req = String::from_utf8_lossy(&buf);
            if req.contains("/meta") {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    meta_json.len(),
                    meta_json
                );
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
            } else {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bpk_clone.len()
                );
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
                let _ = std::io::Write::write_all(&mut stream, &bpk_clone);
            }
        }
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let meta_url = format!("http://{}/meta", addr);
    let art_url = format!("http://{}/art", addr);

    let res = paquetes::instalar_paquete_remoto_completo(&dir, &meta_url, &art_url);
    assert!(res.is_err());
    assert!(paquetes::leer_bloqueo(&dir).unwrap().is_none());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn remoto_e2e_nonce_repetido() {
    let dir = std::env::temp_dir().join(format!("basilisco_e2e_n_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let sk = SigningKey::from_bytes(&[3u8; 32]);
    let vk = sk.verifying_key();
    let vk_bytes = vk.to_bytes().to_vec();
    let vk_hex: String = vk_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    let mut almacen = paquetes::AlmacenClaves::default();
    almacen
        .agregar_seguro("registro1", "key1", vk_bytes.clone())
        .unwrap();
    almacen.guardar(&dir).unwrap();

    let bpk = hacer_bpk();
    let sha = paquetes::sha256_hex(&bpk);

    let mut meta = paquetes::MetadatosPaquete {
        namespace: "demo".to_string(),
        name: "demo".to_string(),
        version: "0.1.0".to_string(),
        sha256: sha.clone(),
        published_at: "2026-10-31T00:00:00Z".to_string(),
        registry_id: "registro1".to_string(),
        key_id: "key1".to_string(),
        nonce: "nonce-n-1".to_string(),
        expires_at: "2027-10-31T00:00:00Z".to_string(),
        firma: None,
        clave: Some(vk_hex.clone()),
    };
    let firma = paquetes::firmar_metadatos(&meta, &sk.to_bytes()).unwrap();
    meta.firma = Some(firma);
    meta.clave = Some(vk_hex);

    let meta_json = serde_json::to_string(&meta).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let bpk_clone = bpk.clone();
    thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = stream.unwrap();
            let mut buf = [0; 1024];
            let _ = stream.read(&mut buf);
            let req = String::from_utf8_lossy(&buf);
            if req.contains("/meta") {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    meta_json.len(),
                    meta_json
                );
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
            } else {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bpk_clone.len()
                );
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
                let _ = std::io::Write::write_all(&mut stream, &bpk_clone);
            }
        }
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let meta_url = format!("http://{}/meta", addr);
    let art_url = format!("http://{}/art", addr);

    assert!(paquetes::instalar_paquete_remoto_completo(&dir, &meta_url, &art_url).is_ok());
    // Segundo intento con mismo nonce debe fallar
    assert!(paquetes::instalar_paquete_remoto_completo(&dir, &meta_url, &art_url).is_err());
    std::fs::remove_dir_all(&dir).ok();
}

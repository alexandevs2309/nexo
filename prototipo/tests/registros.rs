use basilisco::paquetes;
use ed25519_dalek::SigningKey;

#[test]
fn dos_registros_independientes() {
    // Setup dos registros con claves distintas
    let dir = std::env::temp_dir().join(format!("basilisco_reg_{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();

    let sk1 = SigningKey::from_bytes(&[1u8; 32]);
    let vk1 = sk1.verifying_key();
    let vk1_bytes = vk1.to_bytes().to_vec();

    let sk2 = SigningKey::from_bytes(&[2u8; 32]);
    let vk2 = sk2.verifying_key();
    let vk2_bytes = vk2.to_bytes().to_vec();

    // Configurar registros
    let mut cfg = paquetes::ConfigRegistros::default();
    cfg.agregar("reg1", "rid1", "http://127.0.0.1:8001")
        .unwrap();
    cfg.agregar("reg2", "rid2", "http://127.0.0.1:8002")
        .unwrap();
    cfg.set_predeterminado("reg1").unwrap();
    cfg.guardar(&dir).unwrap();

    // Persistencia
    let cfg2 = paquetes::ConfigRegistros::cargar(&dir).unwrap();
    assert_eq!(cfg2.registros.len(), 2);
    assert_eq!(cfg2.predeterminado.as_deref(), Some("reg1"));

    // Almacenes separados por registry_id
    let mut a1 = paquetes::AlmacenClaves::default();
    a1.agregar_seguro("rid1", "k1", vk1_bytes.clone()).unwrap();
    a1.guardar(&dir).unwrap();

    let mut a2_loaded = paquetes::AlmacenClaves::cargar(&dir).unwrap();
    a2_loaded
        .agregar_seguro("rid2", "k2", vk2_bytes.clone())
        .unwrap();
    a2_loaded.guardar(&dir).unwrap();

    // Asegurar que clave de reg1 no autentica reg2
    let a = paquetes::AlmacenClaves::cargar(&dir).unwrap();
    assert!(a.es_confiable("rid1", "k1", &vk1_bytes));
    assert!(a.es_confiable("rid2", "k2", &vk2_bytes));
    assert!(!a.es_confiable("rid1", "k2", &vk2_bytes));
    assert!(!a.es_confiable("rid2", "k1", &vk1_bytes));

    // Eliminar registro sin corrupción
    let mut cfg3 = paquetes::ConfigRegistros::cargar(&dir).unwrap();
    cfg3.eliminar("reg2").unwrap();
    cfg3.guardar(&dir).unwrap();
    let cfg4 = paquetes::ConfigRegistros::cargar(&dir).unwrap();
    assert_eq!(cfg4.registros.len(), 1);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn registro_deshabilitado() {
    let dir = std::env::temp_dir().join(format!("basilisco_reg2_{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();

    let mut cfg = paquetes::ConfigRegistros::default();
    cfg.agregar("r", "rid", "https://example.com").unwrap();
    cfg.habilitar("r", false).unwrap();
    cfg.guardar(&dir).unwrap();

    let cfg2 = paquetes::ConfigRegistros::cargar(&dir).unwrap();
    let r = cfg2.obtener("r").unwrap();
    assert!(!r.habilitado);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn registro_https_requerido() {
    let mut cfg = paquetes::ConfigRegistros::default();
    assert!(cfg.agregar("r", "rid", "http://evil.com").is_err());
    assert!(cfg.agregar("r", "rid", "https://ok.com").is_ok());
}

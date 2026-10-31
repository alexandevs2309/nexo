use std::fs;
use std::process::Command;

fn tmpdir(nombre: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join("bsk_e2e").join(nombre);
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    base
}

fn escribir(dir: &std::path::Path, nombre: &str, contenido: &str) {
    if let Some(parent) = dir.join(nombre).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(dir.join(nombre), contenido).unwrap();
}

fn run_bsk(dir: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_bsk");
    let out = Command::new(bin)
        .args(args)
        .current_dir(dir)
        .output()
        .expect("no se pudo ejecutar");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (code, stdout, stderr)
}

// 1. Importación por alias y llamada a función pública.
#[test]
fn e2e_alias_funcion_publica() {
    let dir = tmpdir("alias_func");
    escribir(
        &dir,
        "lib.bsk",
        "publico funcion doble(n) { retornar n * 2 }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nmostrar lib.doble(5)\n",
    );
    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "10");
}

#[test]
fn e2e_funcion_modulo_sin_retorno_como_sentencia() {
    let dir = tmpdir("funcion_modulo_sin_retorno");
    escribir(
        &dir,
        "lib.bsk",
        "publico funcion saludar() { mostrar \"Hola\" }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nlib.saludar()\nmostrar \"Programa terminado\"\n",
    );

    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "Hola\nPrograma terminado");
}

#[test]
fn e2e_funcion_modulo_sin_retorno_como_valor_conserva_e1010() {
    let dir = tmpdir("funcion_modulo_sin_retorno_valor");
    escribir(
        &dir,
        "lib.bsk",
        "publico funcion saludar() { mostrar \"Hola\" }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nsea resultado = lib.saludar()\n",
    );

    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("E1010"), "stderr: {stderr}");
}

#[test]
fn e2e_funcion_sin_retorno_imprime_y_continua() {
    let dir = tmpdir("funcion_sin_retorno_sentencia");
    escribir(
        &dir,
        "main.bsk",
        "funcion saludar() { mostrar \"Hola\" }\nsaludar()\nmostrar \"Programa terminado\"\n",
    );

    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "Hola\nPrograma terminado");
}

// 2. Acceso a declaración pública por alias.
#[test]
fn e2e_alias_variable_publica() {
    let dir = tmpdir("alias_var");
    escribir(&dir, "lib.bsk", "publico sea PI = 3.14\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nmostrar lib.PI\n",
    );
    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "3.14");
}

// 3. Importación nominal de varios símbodos públicos.
#[test]
fn e2e_importacion_nominal() {
    let dir = tmpdir("nominal");
    escribir(
        &dir,
        "lib.bsk",
        "publico funcion suma(a, b) { retornar a + b }\npublico sea PI = 3.14\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" { suma, PI }\nmostrar suma(2, 3)\nmostrar PI\n",
    );
    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "5\n3.14");
}

// 4. Rechazo de símbodo privado por alias.
#[test]
fn e2e_privado_por_alias() {
    let dir = tmpdir("priv_alias");
    escribir(&dir, "lib.bsk", "funcion secreta() { retornar 42 }\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nmostrar lib.secreta()\n",
    );
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("no es público") || stderr.contains("no existe"),
        "stderr: {stderr}"
    );
}

// 5. Rechazo de símbodo privado por importación nominal.
#[test]
fn e2e_privado_nominal() {
    let dir = tmpdir("priv_nom");
    escribir(&dir, "lib.bsk", "funcion secreta() { retornar 42 }\n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" { secreta }\n");
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("no es público") || stderr.contains("no existe"),
        "stderr: {stderr}"
    );
}

// 6. Rechazo de símbodo inexistente.
#[test]
fn e2e_inexistente() {
    let dir = tmpdir("inexistente");
    escribir(&dir, "lib.bsk", "publico sea PI = 3.14\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nmostrar lib.noExiste\n",
    );
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("no existe"), "stderr: {stderr}");
}

// 7. Conflicto de alias duplicado.
#[test]
fn e2e_alias_duplicado() {
    let dir = tmpdir("alias_dup");
    escribir(&dir, "a.bsk", "publico sea X = 1\n");
    escribir(&dir, "b.bsk", "publico sea Y = 2\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"a.bsk\" como m\nusar \"b.bsk\" como m\n",
    );
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("duplicada") || stderr.contains("duplicado"),
        "stderr: {stderr}"
    );
}

// 8. Conflicto de nombre importado duplicado.
#[test]
fn e2e_nombre_duplicado() {
    let dir = tmpdir("nombre_dup");
    escribir(&dir, "lib.bsk", "publico sea PI = 3.14\n");
    escribir(
        &dir,
        "main.bsk",
        "funcion PI() { retornar 1 }\nusar \"lib.bsk\" { PI }\n",
    );
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("duplicada") || stderr.contains("duplicado"),
        "stderr: {stderr}"
    );
}

// 9. Resolución relativa desde subdirectorio.
#[test]
fn e2e_relativa_subdir() {
    // Sin manifiesto, la raíz es el directorio del archivo de entrada.
    // Por eso ambos archivos deben estar dentro de la misma raíz.
    let dir = tmpdir("relativa");
    escribir(
        &dir,
        "main.bsk",
        "usar \"sub/helper.bsk\" como h\nmostrar h.doble(7)\n",
    );
    escribir(
        &dir,
        "sub/helper.bsk",
        "publico funcion doble(n) { retornar n * 2 }\n",
    );
    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "14");
}

// 10. Ciclo directo.
#[test]
fn e2e_ciclo_directo() {
    let dir = tmpdir("ciclo");
    escribir(&dir, "a.bsk", "usar \"b.bsk\" como b\n");
    escribir(&dir, "b.bsk", "usar \"a.bsk\" como a\n");
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "a.bsk"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("ciclo"), "stderr: {stderr}");
}

// 11. Error de ejecución en módulo con ruta correcta.
#[test]
fn e2e_error_ejecucion_modulo() {
    let dir = tmpdir("err_exec");
    escribir(&dir, "lib.bsk", "publico sea X = 1 / 0\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nmostrar lib.X\n",
    );
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("lib.bsk"), "stderr: {stderr}");
    assert!(stderr.contains("división entre cero"), "stderr: {stderr}");
}

// 12. Error de sintaxis en módulo con ruta correcta.
#[test]
fn e2e_error_sintaxis_modulo() {
    let dir = tmpdir("err_syn");
    escribir(&dir, "lib.bsk", "sea X = \n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" como lib\n");
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("lib.bsk"), "stderr: {stderr}");
}

// 13. Sin efectos secundarios de sentencias arbitrarias en módulo.
#[test]
fn e2e_sin_efectos_secundarios() {
    let dir = tmpdir("sin_efectos");
    // Un módulo con `mostrar` en nivel superior debe ser rechazado.
    escribir(&dir, "lib.bsk", "mostrar \"hola\"\npublico sea X = 1\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib.bsk\" como lib\nmostrar lib.X\n",
    );
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("sentencia no permitida") || stderr.contains("no permitida"),
        "stderr: {stderr}"
    );
}

// 14. Aislamiento entre módulos.
#[test]
fn e2e_aislamiento() {
    let dir = tmpdir("aislamiento");
    escribir(&dir, "a.bsk", "publico sea X = 10\n");
    escribir(&dir, "b.bsk", "publico sea X = 20\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"a.bsk\" como a\nusar \"b.bsk\" como b\nmostrar a.X\nmostrar b.X\n",
    );
    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "10\n20");
}

// 15. Compatibilidad con programa de un solo archivo.
#[test]
fn e2e_compatibilidad_un_archivo() {
    let dir = tmpdir("compat");
    escribir(&dir, "main.bsk", "sea x = 5\nmostrar x * 2\n");
    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "10");
}

// 16. Ruta fuera del proyecto.
#[test]
fn e2e_ruta_fuera() {
    let dir = tmpdir("ruta_fuera");
    let fuera = tmpdir("ruta_fuera_ext");
    escribir(&fuera, "secreto.bsk", "publico sea X = 1\n");
    escribir(
        &dir,
        "main.bsk",
        "usar \"../ruta_fuera_ext/secreto.bsk\" como lib\n",
    );
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("ruta no permitida") || stderr.contains("no permitida"),
        "stderr: {stderr}"
    );
}

// 17. Archivo inexistente.
#[test]
fn e2e_archivo_inexistente() {
    let dir = tmpdir("inexistente");
    escribir(&dir, "main.bsk", "usar \"no_existe.bsk\" como lib\n");
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("módulo no encontrado") || stderr.contains("no encontrado"),
        "stderr: {stderr}"
    );
}

// 18. Importación transitiva A → B → C.
#[test]
fn e2e_transitiva() {
    let dir = tmpdir("transitiva");
    escribir(
        &dir,
        "c.bsk",
        "publico funcion triple(n) { retornar n * 3 }\n",
    );
    escribir(
        &dir,
        "b.bsk",
        "usar \"c.bsk\" como c\npublico funcion doble(n) { retornar c.triple(n) + n }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"b.bsk\" como b\nmostrar b.doble(5)\n",
    );
    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "20");
}

// 19. Ciclo indirecto A → B → C → A.
#[test]
fn e2e_ciclo_indirecto() {
    let dir = tmpdir("ciclo_indirecto");
    escribir(&dir, "a.bsk", "usar \"b.bsk\" como b\n");
    escribir(&dir, "b.bsk", "usar \"c.bsk\" como c\n");
    escribir(&dir, "c.bsk", "usar \"a.bsk\" como a\n");
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "a.bsk"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("ciclo"), "stderr: {stderr}");
}

// 20. Error léxico en módulo con ruta correcta.
#[test]
fn e2e_error_lexico_modulo() {
    let dir = tmpdir("err_lex");
    escribir(&dir, "lib.bsk", "sea X = \"sin cerrar\n");
    escribir(&dir, "main.bsk", "usar \"lib.bsk\" como lib\n");
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("lib.bsk"), "stderr: {stderr}");
}

// 21. Rechazo de extensión no permitida.
#[test]
fn e2e_extension_no_permitida() {
    let dir = tmpdir("ext_no");
    escribir(&dir, "lib.nx", "publico sea X = 1\n");
    escribir(&dir, "main.bsk", "usar \"lib.nx\" como lib\n");
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("extensión") || stderr.contains("no soportada"),
        "stderr: {stderr}"
    );
}

// 22. Combinación de rutas para escapar.
#[test]
fn e2e_combinacion_escape() {
    let dir = tmpdir("combo_escape");
    let fuera = tmpdir("combo_escape_ext");
    escribir(&fuera, "secreto.bsk", "publico sea X = 1\n");
    fs::create_dir_all(dir.join("sub")).unwrap();
    escribir(
        &dir,
        "main.bsk",
        "usar \"sub/../../combo_escape_ext/secreto.bsk\" como lib\n",
    );
    let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("ruta no permitida")
            || stderr.contains("no permitida")
            || stderr.contains("módulo no encontrado"),
        "stderr: {stderr}"
    );
}

// 23. Importación válida desde subdirectorio autorizado.
#[test]
fn e2e_subdir_valido() {
    let dir = tmpdir("subdir_valido");
    escribir(
        &dir,
        "lib/sub.bsk",
        "publico funcion inc(n) { retornar n + 1 }\n",
    );
    escribir(
        &dir,
        "main.bsk",
        "usar \"lib/sub.bsk\" como s\nmostrar s.inc(5)\n",
    );
    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "6");
}

// 24. Symlink que apunta fuera de la raíz (solo en Unix).
#[test]
fn e2e_symlink_escape() {
    let dir = tmpdir("sym_escape");
    let fuera = tmpdir("sym_escape_ext");
    escribir(&fuera, "secreto.bsk", "publico sea X = 1\n");
    fs::create_dir_all(dir.join("sub")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let _ = symlink(fuera.join("secreto.bsk"), dir.join("escape.bsk"));
        escribir(
            &dir,
            "main.bsk",
            "usar \"escape.bsk\" como lib\nmostrar lib.X\n",
        );
        let (code, _stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
        assert_ne!(code, 0, "symlink escape debería ser rechazado: {stderr}");
    }
}

// 25. Sin extensión se añade .bsk.
#[test]
fn e2e_sin_extension() {
    let dir = tmpdir("sin_ext");
    escribir(&dir, "lib.bsk", "publico sea X = 42\n");
    escribir(&dir, "main.bsk", "usar \"lib\" como lib\nmostrar lib.X\n");
    let (code, stdout, stderr) = run_bsk(&dir, &["run", "main.bsk"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "42");
}

//! VM (máquina virtual) de BASILISCO.
//!
//! Intérprete directo de MIR: ejecuta funciones MIR bloques-a-bloques con
//! terminadores explícitos, reutilizando `Valor` (la semántica aritmética del
//! núcleo: ADR-0005, `lógico` ADR-0007) y sus métodos (`suma`, `resta`, ...).
//!
//! Alcance de esta versión de andamiaje:
//! - Cada llamada es un marco recursivo con límite de profundidad.
//! - Contador de pasos compartido (`limite_pasos`) para detectar bucles infinitos.
//! - Errores de ejecución en `ErrorVm`; la aritmética reutiliza `MotivoAritmetico`.
//! - Constantes MIR ↔ `Valor` por conversión directa; los tipos ricos (arreglos,
//!   listas, objetos) se materializan como `Valor` completos.
//!
//! Futuro: bytecode real, marcos planos (no recurrentes), llamadas nativas FFI,
//! proyecciones anidadas en `MirPlace`.

use std::collections::HashMap;

use crate::mir::{
    BlockId, MirBinOp, MirConst, MirOperand, MirPlace, MirPrograma, MirProjection, MirRvalue,
    MirStmt, MirTerminator, MirUnOp,
};
use crate::valor::{FACTOR_ESCALA, Valor};

/// Error de ejecución de la VM.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorVm {
    /// La función llamada no existe en el programa.
    FuncionNoEncontrada(String),
    /// La función se llamó con menos argumentos que parámetros.
    ArgumentosInsuficientes { faltantes: usize },
    /// Se alcanzó el límite de pasos (posible bucle infinito).
    LimitePasosExcedido,
    /// Se alcanzó el límite de profundidad de llamadas (posible recursión infinita).
    ProfundidadExcedida,
    /// Se llegó a `Unreachable` o a un salto fuera del mapa de bloques.
    BloqueInalcanzable,
    /// Error aritmético reutilizando los motivos del núcleo (ADR-0005 `D2`-`D5`).
    Aritmetico(crate::valor::MotivoAritmetico),
    /// Operación entre tipos incompatibles.
    TiposIncompatibles {
        izquierdo: &'static str,
        derecho: &'static str,
    },
    /// Se esperaba un tipo concreto (p. ej. índice entero).
    TipoEsperado {
        esperado: &'static str,
        encontrado: &'static str,
    },
    /// Acceso fuera de los límites de un arreglo/lista.
    IndiceFueraDeRango { indice: i64, tamano: usize },
    /// El campo no existe en el objeto.
    CampoInexistente { campo: String },
}

impl std::fmt::Display for ErrorVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorVm::FuncionNoEncontrada(nombre) => write!(f, "función no encontrada: {nombre}"),
            ErrorVm::ArgumentosInsuficientes { faltantes } => {
                write!(f, "argumentos insuficientes ({faltantes} faltantes)")
            }
            ErrorVm::LimitePasosExcedido => write!(f, "límite de pasos excedido"),
            ErrorVm::ProfundidadExcedida => write!(f, "límite de profundidad excedido"),
            ErrorVm::BloqueInalcanzable => write!(f, "bloque inalcanzable"),
            ErrorVm::Aritmetico(m) => write!(f, "error aritmético: {m}"),
            ErrorVm::TiposIncompatibles { izquierdo, derecho } => {
                write!(f, "tipos incompatibles: {izquierdo} y {derecho}")
            }
            ErrorVm::TipoEsperado {
                esperado,
                encontrado,
            } => {
                write!(f, "se esperaba «{esperado}», se encontró «{encontrado}»")
            }
            ErrorVm::IndiceFueraDeRango { indice, tamano } => {
                write!(f, "índice {indice} fuera de rango (tamaño {tamano})")
            }
            ErrorVm::CampoInexistente { campo } => write!(f, "campo inexistente: {campo}"),
        }
    }
}

impl std::error::Error for ErrorVm {}

/// Límite por defecto de pasos de ejecución (sentencias + terminadores).
pub const LIMITE_PASOS_DEFAULT: usize = 1_000_000;
/// Límite por defecto de profundidad de llamadas.
pub const LIMITE_PROFUNDIDAD_DEFAULT: usize = 1_000;

/// Tipo de una función nativa registrada en la VM: recibe los argumentos
/// evaluados y devuelve un `Valor` (o un `ErrorVm`).
pub type FuncionNativa = std::sync::Arc<dyn Fn(&[Valor]) -> Result<Valor, ErrorVm> + Send + Sync>;

/// Máquina virtual. Interpreta un `MirPrograma`.
pub struct MaquinaVirtual {
    programa: MirPrograma,
    /// Índice `nombre -> posición en programa.funciones` (la primera gana).
    indice: HashMap<String, usize>,
    /// Funciones nativas (FFI / integradas) registradas en la VM.
    nativos: HashMap<String, FuncionNativa>,
    /// Salida capturada de `mostrar`/`mostrar_sin_salto` (biblioteca estándar).
    salida: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    pasos: usize,
    limite_pasos: usize,
    profundidad: usize,
    limite_profundidad: usize,
}

impl MaquinaVirtual {
    /// Crea una VM con los límites por defecto.
    pub fn nuevo(programa: MirPrograma) -> Self {
        MaquinaVirtual::con_limites(programa, LIMITE_PASOS_DEFAULT, LIMITE_PROFUNDIDAD_DEFAULT)
    }

    /// Crea una VM con límites personalizados (útil en tests).
    pub fn con_limites(
        programa: MirPrograma,
        limite_pasos: usize,
        limite_profundidad: usize,
    ) -> Self {
        let mut indice = HashMap::new();
        for (i, f) in programa.funciones.iter().enumerate() {
            indice.entry(f.nombre.clone()).or_insert(i);
        }
        MaquinaVirtual {
            programa,
            indice,
            nativos: HashMap::new(),
            salida: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            pasos: 0,
            limite_pasos,
            profundidad: 0,
            limite_profundidad,
        }
    }

    /// Número de funciones del programa.
    pub fn numero_funciones(&self) -> usize {
        self.programa.funciones.len()
    }

    /// Índice de la función por nombre (`f_{simbodo}` según el lowering).
    pub fn indice_de(&self, nombre: &str) -> Option<usize> {
        self.indice.get(nombre).copied()
    }

    /// Registra una función nativa (FFI / integrada) bajo `nombre`.
    /// Tiene prioridad menor que una función MIR del programa con el mismo
    /// nombre (la primera gana en el intérprete).
    pub fn registrar_nativo(&mut self, nombre: impl Into<String>, f: FuncionNativa) {
        self.nativos.entry(nombre.into()).or_insert(f);
    }

    /// Registra la biblioteca estándar mínima (mostrar, mostrar_sin_salto).
    pub fn registrar_stdlib(&mut self) {
        let salida = std::sync::Arc::clone(&self.salida);
        self.registrar_nativo(
            "mostrar",
            std::sync::Arc::new(move |args: &[Valor]| {
                let texto = args.first().map(|v| v.to_string()).unwrap_or_default();
                salida.lock().unwrap().push(format!("{texto}\n"));
                println!("{texto}");
                Ok(Valor::Nulo)
            }),
        );
        let salida = std::sync::Arc::clone(&self.salida);
        self.registrar_nativo(
            "mostrar_sin_salto",
            std::sync::Arc::new(move |args: &[Valor]| {
                let texto = args.first().map(|v| v.to_string()).unwrap_or_default();
                salida.lock().unwrap().push(texto.clone());
                print!("{texto}");
                Ok(Valor::Nulo)
            }),
        );
    }

    /// Salida capturada de mostrar/mostrar_sin_salto.
    pub fn salida(&self) -> Vec<String> {
        self.salida.lock().unwrap().clone()
    }

    /// Número de funciones nativas registradas.
    pub fn numero_nativos(&self) -> usize {
        self.nativos.len()
    }

    /// Ejecuta la función `nombre` con `args` y devuelve su valor de retorno.
    pub fn ejecutar(&mut self, nombre: &str, args: &[Valor]) -> Result<Option<Valor>, ErrorVm> {
        let idx = self
            .indice_de(nombre)
            .ok_or_else(|| ErrorVm::FuncionNoEncontrada(nombre.to_string()))?;
        self.ejecutar_indice(idx, args)
    }

    /// Ejecuta la función en la posición `idx` de `programa.funciones`.
    pub fn ejecutar_indice(
        &mut self,
        idx: usize,
        args: &[Valor],
    ) -> Result<Option<Valor>, ErrorVm> {
        // Se clona la función para evitar préstamos cruzados con el estado de la
        // VM (los programas del prototipo son pequeños; en bytecode real el
        // código será compartido, no clonado).
        let Some(funcion) = self.programa.funciones.get(idx).cloned() else {
            return Err(ErrorVm::BloqueInalcanzable);
        };
        let r = self.correr_funcion(&funcion, args);
        self.profundidad -= 1;
        r
    }

    /// Registra un paso; falla si se excede el límite.
    fn paso(&mut self) -> Result<(), ErrorVm> {
        self.pasos += 1;
        if self.pasos > self.limite_pasos {
            return Err(ErrorVm::LimitePasosExcedido);
        }
        Ok(())
    }

    /// Bucle principal de la función: recorre bloques hasta un `Return`.
    fn correr_funcion(
        &mut self,
        f: &crate::mir::MirFunction,
        args: &[Valor],
    ) -> Result<Option<Valor>, ErrorVm> {
        self.profundidad += 1;
        if self.profundidad > self.limite_profundidad {
            return Err(ErrorVm::ProfundidadExcedida);
        }

        // Locales: ceros-nulos y parámetros asignados por posición.
        let mut locals: Vec<Valor> = vec![Valor::Nulo; f.locals.len()];
        if args.len() < f.params.len() {
            return Err(ErrorVm::ArgumentosInsuficientes {
                faltantes: f.params.len() - args.len(),
            });
        }
        for (i, (local, _ty)) in f.params.iter().enumerate() {
            locals[local.0] = args[i].clone();
        }

        let mut bloque_actual = f.bloque_entrada;
        loop {
            self.paso()?;
            if bloque_actual.0 >= f.bloques.len() {
                return Err(ErrorVm::BloqueInalcanzable);
            }
            let bloque = &f.bloques[bloque_actual.0];

            // Sentencias del bloque.
            for stmt in &bloque.stmts {
                self.paso()?;
                self.ejecutar_stmt(&mut locals, stmt)?;
            }

            // Terminador: salto al siguiente bloque o retorno.
            match self.ejecutar_terminador(&mut locals, &bloque.terminator)? {
                Transicion::Salto(nuevo) => bloque_actual = nuevo,
                Transicion::Retornar(valor) => return Ok(valor),
            }
        }
    }

    /// Ejecuta una sentencia no-terminadora.
    fn ejecutar_stmt(&mut self, locals: &mut [Valor], stmt: &MirStmt) -> Result<(), ErrorVm> {
        match stmt {
            MirStmt::Asignacion { destino, valor } => {
                let v = self.evaluar_rvalue(locals, valor)?;
                self.escribir_place(locals, destino, v)
            }
            MirStmt::Llamada {
                destino,
                callee,
                argumentos,
            } => {
                let args = self.evaluar_argumentos(locals, argumentos)?;
                let r = self.llamar(callee, &args)?;
                if let Some(dest) = destino {
                    self.escribir_place(locals, dest, r)?;
                }
                Ok(())
            }
            MirStmt::LlamadaIndirecta {
                destino,
                callee,
                argumentos,
            } => {
                let callee_val = self.operando_a_valor(locals, callee)?;
                let nombre = match &callee_val {
                    Valor::Funcion(n) => n.clone(),
                    Valor::ClausuraMir { nombre, .. } => nombre.clone(),
                    otro => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "función",
                            encontrado: otro.nombre_tipo(),
                        });
                    }
                };
                let mut args = self.evaluar_argumentos(locals, argumentos)?;
                if let Valor::ClausuraMir { capturas, .. } = &callee_val {
                    let mut full = args.clone();
                    full.extend(capturas.clone());
                    args = full;
                }
                let r = self.llamar(&nombre, &args)?;
                if let Some(dest) = destino {
                    self.escribir_place(locals, dest, r)?;
                }
                Ok(())
            }
            MirStmt::Nop => Ok(()),
        }
    }

    /// Evalúa los argumentos de una llamada.
    fn evaluar_argumentos(
        &self,
        locals: &[Valor],
        ops: &[MirOperand],
    ) -> Result<Vec<Valor>, ErrorVm> {
        ops.iter()
            .map(|op| self.operando_a_valor(locals, op))
            .collect()
    }

    /// Ejecuta un terminador y devuelve la transición (salto o retorno).
    fn ejecutar_terminador(
        &mut self,
        locals: &mut [Valor],
        t: &MirTerminator,
    ) -> Result<Transicion, ErrorVm> {
        match t {
            MirTerminator::Goto(b) => Ok(Transicion::Salto(*b)),
            MirTerminator::SwitchInt {
                discriminante,
                casos,
                por_defecto,
            } => {
                let v = self.operando_a_valor(locals, discriminante)?;
                let disc = valor_a_discriminante(&v)?;
                for (valor, b) in casos {
                    if disc == *valor {
                        return Ok(Transicion::Salto(*b));
                    }
                }
                Ok(Transicion::Salto(*por_defecto))
            }
            MirTerminator::Return(op) => match op {
                Some(op) => {
                    let v = self.operando_a_valor(locals, op)?;
                    Ok(Transicion::Retornar(Some(v)))
                }
                None => Ok(Transicion::Retornar(None)),
            },
            MirTerminator::Invoke {
                callee,
                argumentos,
                destino,
                bloque_ok,
                bloque_error,
            } => {
                let args = self.evaluar_argumentos(locals, argumentos)?;
                match self.llamar(callee, &args) {
                    Ok(v) => {
                        if let Some(dest) = destino {
                            self.escribir_place(locals, dest, v)?;
                        }
                        Ok(Transicion::Salto(*bloque_ok))
                    }
                    Err(_) => Ok(Transicion::Salto(*bloque_error)),
                }
            }
            MirTerminator::Unreachable => Err(ErrorVm::BloqueInalcanzable),
        }
    }

    /// Llama a una función MIR por nombre. Sin retorno MIR equivale a nulo.
    fn llamar(&mut self, callee: &str, args: &[Valor]) -> Result<Valor, ErrorVm> {
        // Funciones MIR del programa tienen prioridad.
        if self.indice.contains_key(callee) {
            let devuelto = self.ejecutar(callee, args)?;
            return Ok(devuelto.unwrap_or(Valor::Nulo));
        }
        // Fallback: función nativa registrada (FFI / integrada).
        if self.nativos.contains_key(callee) {
            self.paso()?;
            let f = self.nativos.get(callee).cloned().unwrap();
            return f(args);
        }
        Err(ErrorVm::FuncionNoEncontrada(callee.to_string()))
    }

    /// Lee un operando: clona el local o materializa la constante.
    fn operando_a_valor(&self, locals: &[Valor], op: &MirOperand) -> Result<Valor, ErrorVm> {
        match op {
            MirOperand::Copy(local) | MirOperand::Move(local) => {
                locals.get(local.0).cloned().ok_or(ErrorVm::TipoEsperado {
                    esperado: "local",
                    encontrado: "vacío",
                })
            }
            MirOperand::Const(c) => Ok(const_a_valor(c)),
        }
    }

    /// Escribe en un `MirPlace` (local + proyecciones de un solo nivel).
    fn escribir_place(
        &self,
        locals: &mut [Valor],
        place: &MirPlace,
        valor: Valor,
    ) -> Result<(), ErrorVm> {
        let indice = match place.projection.as_slice() {
            [MirProjection::Indice(op)] => {
                let Valor::Entero(i) = self.operando_a_valor(locals, op)? else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "entero",
                        encontrado: "índice no entero",
                    });
                };
                if i < 0 {
                    return Err(ErrorVm::IndiceFueraDeRango {
                        indice: i,
                        tamano: 0,
                    });
                }
                Some(i as usize)
            }
            _ => None,
        };
        let Some(local_val) = locals.get_mut(place.local.0) else {
            return Err(ErrorVm::TipoEsperado {
                esperado: "local",
                encontrado: "vacío",
            });
        };
        match place.projection.as_slice() {
            [] => {
                *local_val = valor;
                Ok(())
            }
            [MirProjection::Campo(campo)] => match local_val {
                Valor::Objeto(mapa) => {
                    mapa.insert(campo.clone(), valor);
                    Ok(())
                }
                otro => Err(ErrorVm::TipoEsperado {
                    esperado: "objeto",
                    encontrado: otro.nombre_tipo(),
                }),
            },
            [MirProjection::Indice(_)] => {
                let idx = indice.expect("resuelto arriba");
                match local_val {
                    Valor::Arreglo(elems) | Valor::Lista(elems) => {
                        if idx >= elems.len() {
                            return Err(ErrorVm::IndiceFueraDeRango {
                                indice: idx as i64,
                                tamano: elems.len(),
                            });
                        }
                        elems[idx] = valor;
                        Ok(())
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "arreglo o lista",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            [MirProjection::Celda] => match local_val {
                Valor::Celda(c) => {
                    let mut guard = c.lock().unwrap();
                    *guard = valor;
                    Ok(())
                }
                otro => Err(ErrorVm::TipoEsperado {
                    esperado: "celda",
                    encontrado: otro.nombre_tipo(),
                }),
            },
            _ => Err(ErrorVm::TipoEsperado {
                esperado: "proyección simple",
                encontrado: "proyección anidada",
            }),
        }
    }

    /// Evalúa un rvalue.
    fn evaluar_rvalue(&self, locals: &[Valor], r: &MirRvalue) -> Result<Valor, ErrorVm> {
        match r {
            MirRvalue::Use(op) => self.operando_a_valor(locals, op),
            MirRvalue::BinOp { op, lhs, rhs } => {
                let a = self.operando_a_valor(locals, lhs)?;
                let b = self.operando_a_valor(locals, rhs)?;
                aplicar_binop(op, &a, &b)
            }
            MirRvalue::UnOp { op, operand } => {
                let a = self.operando_a_valor(locals, operand)?;
                aplicar_unop(op, &a)
            }
            MirRvalue::Agregado { elementos } => {
                let elems: Result<Vec<Valor>, _> = elementos
                    .iter()
                    .map(|op| self.operando_a_valor(locals, op))
                    .collect();
                Ok(Valor::Arreglo(elems?))
            }
            MirRvalue::Indice { base, indice } => {
                let base_v = self.operando_a_valor(locals, base)?;
                let idx_v = self.operando_a_valor(locals, indice)?;
                let Valor::Entero(idx) = idx_v else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "entero",
                        encontrado: "índice no entero",
                    });
                };
                match &base_v {
                    Valor::Arreglo(elems) | Valor::Lista(elems) => {
                        if idx < 0 || idx as usize >= elems.len() {
                            return Err(ErrorVm::IndiceFueraDeRango {
                                indice: idx,
                                tamano: elems.len(),
                            });
                        }
                        Ok(elems[idx as usize].clone())
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "arreglo o lista",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            MirRvalue::Campo { base, campo } => {
                let base_v = self.operando_a_valor(locals, base)?;
                match &base_v {
                    Valor::Objeto(mapa) => {
                        mapa.get(campo)
                            .cloned()
                            .ok_or_else(|| ErrorVm::CampoInexistente {
                                campo: campo.clone(),
                            })
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "objeto",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            MirRvalue::CrearCelda { valor } => {
                let v = self.operando_a_valor(locals, valor)?;
                Ok(Valor::Celda(std::sync::Arc::new(std::sync::Mutex::new(v))))
            }
            MirRvalue::LeerCelda { base } => {
                let base_v = self.operando_a_valor(locals, base)?;
                match &base_v {
                    Valor::Celda(c) => {
                        let guard = c.lock().unwrap();
                        Ok(guard.clone())
                    }
                    otro => Err(ErrorVm::TipoEsperado {
                        esperado: "celda",
                        encontrado: otro.nombre_tipo(),
                    }),
                }
            }
            MirRvalue::Clausura { nombre, capturas } => {
                let caps: Vec<Valor> = capturas
                    .iter()
                    .map(|op| self.operando_a_valor(locals, op))
                    .collect::<Result<_, _>>()?;
                Ok(Valor::ClausuraMir {
                    nombre: nombre.clone(),
                    capturas: caps,
                })
            }
            MirRvalue::Longitud { base } => {
                let base_v = self.operando_a_valor(locals, base)?;
                let n = match &base_v {
                    Valor::Arreglo(e) | Valor::Lista(e) => e.len() as i64,
                    Valor::Texto(s) => s.chars().count() as i64,
                    otro => {
                        return Err(ErrorVm::TipoEsperado {
                            esperado: "arreglo, lista o texto",
                            encontrado: otro.nombre_tipo(),
                        });
                    }
                };
                Ok(Valor::Entero(n))
            }
            MirRvalue::Rango {
                inicio,
                fin,
                inclusivo,
            } => {
                let vi = self.operando_a_valor(locals, inicio)?;
                let vf = self.operando_a_valor(locals, fin)?;
                let (Valor::Entero(a), Valor::Entero(b)) = (vi, vf) else {
                    return Err(ErrorVm::TipoEsperado {
                        esperado: "entero",
                        encontrado: "rango no entero",
                    });
                };
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                let mut elems = Vec::new();
                if *inclusivo {
                    let mut i = lo;
                    while i <= hi {
                        elems.push(Valor::Entero(i));
                        i += 1;
                    }
                } else {
                    let mut i = lo;
                    while i < hi {
                        elems.push(Valor::Entero(i));
                        i += 1;
                    }
                }
                Ok(Valor::Lista(elems))
            }
        }
    }
}

/// Transición de un terminador: salto a bloque o retorno.
enum Transicion {
    Salto(BlockId),
    Retornar(Option<Valor>),
}

/// Convierte un `MirConst` en `Valor`.
fn const_a_valor(c: &MirConst) -> Valor {
    match c {
        MirConst::Entero(n) => Valor::Entero(*n),
        MirConst::Decimal(n) => Valor::Decimal(*n),
        MirConst::Real(n) => Valor::Real(*n),
        MirConst::Logico(b) => Valor::Logico(*b),
        MirConst::Texto(s) => Valor::Texto(s.clone()),
        MirConst::Nulo => Valor::Nulo,
    }
}

/// Discriminante (i64) para `SwitchInt`: lógicos 0/1, enteros como tal.
fn valor_a_discriminante(v: &Valor) -> Result<i64, ErrorVm> {
    match v {
        Valor::Entero(n) => Ok(*n),
        Valor::Logico(b) => Ok(i64::from(*b)),
        Valor::Nulo => Ok(0),
        otro => Err(ErrorVm::TipoEsperado {
            esperado: "entero o lógico",
            encontrado: otro.nombre_tipo(),
        }),
    }
}

/// Aplica un operador binario MIR con la semántica de `Valor` del núcleo.
fn aplicar_binop(op: &MirBinOp, a: &Valor, b: &Valor) -> Result<Valor, ErrorVm> {
    match op {
        MirBinOp::Suma => a.suma(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Resta => a.resta(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Multiplicacion => a.multiplicacion(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Division => a.division(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Modulo => a.modulo(b).map_err(ErrorVm::Aritmetico),
        MirBinOp::Igual => Ok(Valor::Logico(a == b)),
        MirBinOp::Distinto => Ok(Valor::Logico(a != b)),
        MirBinOp::Menor => comparar(a, b, |o| o.is_lt()),
        MirBinOp::MenorIgual => comparar(a, b, |o| o.is_le()),
        MirBinOp::Mayor => comparar(a, b, |o| o.is_gt()),
        MirBinOp::MayorIgual => comparar(a, b, |o| o.is_ge()),
        MirBinOp::Y | MirBinOp::O => match (a, b) {
            (Valor::Logico(x), Valor::Logico(y)) => {
                let r = if *op == MirBinOp::Y {
                    *x && *y
                } else {
                    *x || *y
                };
                Ok(Valor::Logico(r))
            }
            _ => Err(ErrorVm::TiposIncompatibles {
                izquierdo: a.nombre_tipo(),
                derecho: b.nombre_tipo(),
            }),
        },
    }
}

/// Comparaciones numéricas/texto (entero↔decimal/real implícito, ADR-0005 §1.4).
fn comparar(
    a: &Valor,
    b: &Valor,
    acepta: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<Valor, ErrorVm> {
    let orden = match (a, b) {
        (Valor::Entero(x), Valor::Entero(y)) => x.cmp(y),
        (Valor::Entero(x), Valor::Decimal(y)) => (*x as i128 * FACTOR_ESCALA).cmp(y),
        (Valor::Decimal(x), Valor::Entero(y)) => x.cmp(&(*y as i128 * FACTOR_ESCALA)),
        (Valor::Decimal(x), Valor::Decimal(y)) => x.cmp(y),
        (Valor::Real(x), Valor::Real(y)) => match x.partial_cmp(y) {
            Some(o) => o,
            // NaN: ninguna relación de orden es cierta (semántica de `evaluador`).
            None => return Ok(Valor::Logico(false)),
        },
        (Valor::Texto(x), Valor::Texto(y)) => x.cmp(y),
        _ => {
            return Err(ErrorVm::TiposIncompatibles {
                izquierdo: a.nombre_tipo(),
                derecho: b.nombre_tipo(),
            });
        }
    };
    Ok(Valor::Logico(acepta(orden)))
}

/// Aplica un operador unario MIR.
fn aplicar_unop(op: &MirUnOp, a: &Valor) -> Result<Valor, ErrorVm> {
    match op {
        MirUnOp::No => match a {
            Valor::Logico(b) => Ok(Valor::Logico(!*b)),
            otro => Err(ErrorVm::TipoEsperado {
                esperado: "lógico",
                encontrado: otro.nombre_tipo(),
            }),
        },
        MirUnOp::Negacion => match a {
            Valor::Entero(n) => n
                .checked_neg()
                .map(Valor::Entero)
                .ok_or(ErrorVm::Aritmetico(
                    crate::valor::MotivoAritmetico::DesbordamientoEntero,
                )),
            Valor::Decimal(m) => Ok(Valor::Decimal(*m * -1)),
            Valor::Real(x) => Ok(Valor::Real(-x)),
            otro => Err(ErrorVm::TiposIncompatibles {
                izquierdo: otro.nombre_tipo(),
                derecho: "negación",
            }),
        },
    }
}

//! Infraestructura de símbodos e identificadores estables para el análisis
//! semántico de BASILISCO (ADR-0017, Etapa 1).
//!
//! Esta etapa introduce únicamente la infraestructura: identificadores estables,
//! tabla de símbodos y gestión de ámbitos. No modifica el AST ni el evaluador.

use std::collections::HashMap;

/// Identificador estable de un símbodo dentro de un programa analizado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SimbodoId(pub u32);

/// Identificador estable de un ámbito.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AmbitoId(pub u32);

/// Categoría de un símbodo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoriaSimbodo {
    Variable,
    Funcion,
    Parametro,
    Modulo,
    Builtin,
}

/// Información de un símbodo. No contiene referencias al AST.
#[derive(Debug, Clone, PartialEq)]
pub struct Simbodo {
    pub id: SimbodoId,
    pub nombre: String,
    pub ambito: AmbitoId,
    pub categoria: CategoriaSimbodo,
    pub mutable: bool,
}

/// Tabla de símbodos de un programa.
#[derive(Debug, Default, Clone)]
pub struct TablaSimbodo {
    simbodos: HashMap<SimbodoId, Simbodo>,
    por_nombre: HashMap<(AmbitoId, String), SimbodoId>,
    siguiente_id: u32,
}

impl TablaSimbodo {
    pub fn nuevo() -> Self {
        Self::default()
    }

    pub fn registrar(
        &mut self,
        nombre: String,
        ambito: AmbitoId,
        categoria: CategoriaSimbodo,
        mutable: bool,
    ) -> SimbodoId {
        let id = SimbodoId(self.siguiente_id);
        self.siguiente_id = self.siguiente_id.saturating_add(1);
        let s = Simbodo {
            id,
            nombre: nombre.clone(),
            ambito,
            categoria,
            mutable,
        };
        self.simbodos.insert(id, s);
        self.por_nombre.insert((ambito, nombre), id);
        id
    }

    pub fn obtener(&self, id: SimbodoId) -> Option<&Simbodo> {
        self.simbodos.get(&id)
    }

    pub fn buscar(&self, ambito: AmbitoId, nombre: &str) -> Option<SimbodoId> {
        self.por_nombre.get(&(ambito, nombre.to_string())).copied()
    }

    pub fn cantidad(&self) -> usize {
        self.simbodos.len()
    }
}

/// Gestor de ámbitos como pila.
#[derive(Debug, Clone)]
pub struct GestorAmbitos {
    ámbitos: Vec<AmbitoId>,
    siguiente_ambito: u32,
}

impl GestorAmbitos {
    pub fn nuevo() -> Self {
        let root = AmbitoId(0);
        Self {
            ámbitos: vec![root],
            siguiente_ambito: 1,
        }
    }

    pub fn actual(&self) -> AmbitoId {
        *self.ámbitos.last().expect("siempre hay ámbito raíz")
    }

    pub fn entrar(&mut self) -> AmbitoId {
        let id = AmbitoId(self.siguiente_ambito);
        self.siguiente_ambito = self.siguiente_ambito.saturating_add(1);
        self.ámbitos.push(id);
        id
    }

    pub fn salir(&mut self) {
        if self.ámbitos.len() > 1 {
            self.ámbitos.pop();
        }
    }

    pub fn profundidad(&self) -> usize {
        self.ámbitos.len()
    }

    pub fn pila(&self) -> &[AmbitoId] {
        &self.ámbitos
    }

    pub fn raiz() -> AmbitoId {
        AmbitoId(0)
    }
}

impl Default for GestorAmbitos {
    fn default() -> Self {
        Self::nuevo()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_estables_y_unicos() {
        let mut t = TablaSimbodo::nuevo();
        let a = t.registrar("x".into(), AmbitoId(0), CategoriaSimbodo::Variable, true);
        let b = t.registrar("y".into(), AmbitoId(0), CategoriaSimbodo::Variable, false);
        assert_ne!(a, b);
        assert_eq!(t.cantidad(), 2);
        assert!(t.obtener(a).is_some());
    }

    #[test]
    fn buscar_por_ambito_y_nombre() {
        let mut t = TablaSimbodo::nuevo();
        let id = t.registrar("f".into(), AmbitoId(1), CategoriaSimbodo::Funcion, false);
        assert_eq!(t.buscar(AmbitoId(1), "f"), Some(id));
        assert_eq!(t.buscar(AmbitoId(0), "f"), None);
    }

    #[test]
    fn gestor_ambitos_pila() {
        let mut g = GestorAmbitos::nuevo();
        assert_eq!(g.actual(), AmbitoId(0));
        let a = g.entrar();
        assert_eq!(g.actual(), a);
        assert_eq!(g.profundidad(), 2);
        g.salir();
        assert_eq!(g.actual(), AmbitoId(0));
        // no salir de raíz
        g.salir();
        assert_eq!(g.actual(), AmbitoId(0));
    }

    #[test]
    fn categoria_y_mutabilidad() {
        let mut t = TablaSimbodo::nuevo();
        let id = t.registrar("p".into(), AmbitoId(0), CategoriaSimbodo::Parametro, false);
        let s = t.obtener(id).unwrap();
        assert_eq!(s.categoria, CategoriaSimbodo::Parametro);
        assert!(!s.mutable);
    }
}

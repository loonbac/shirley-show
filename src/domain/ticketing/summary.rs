use serde::{Deserialize, Serialize};

use crate::domain::ticketing::money::Money;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PaymentMethodBreakdown {
    pub yape_cobrado: Money,
    pub yape_pendiente: Money,
    pub yape_entradas: u32,
    pub efectivo_cobrado: Money,
    pub efectivo_pendiente: Money,
    pub efectivo_entradas: u32,
    pub cortesia_entradas: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExpenseCategoryBreakdown {
    pub local_alquiler: Money,
    pub sonido_luces: Money,
    pub musicos_personal: Money,
    pub publicidad_marketing: Money,
    pub bebidas_catering: Money,
    pub transporte: Money,
    pub otros: Money,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventSummary {
    pub cobrado: Money,
    pub pendiente: Money,
    pub total_proyectado: Money,
    pub gastos: Money,
    pub ganancia_neta: i64,
    pub ganancia_neta_soles: f64,
    pub capacidad_total: u32,
    pub entradas_emitidas: u32,
    pub aforo_proyectado: u32,
    pub metodos_pago: PaymentMethodBreakdown,
    pub categorias_gasto: ExpenseCategoryBreakdown,
}

impl EventSummary {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cobrado: Money,
        pendiente: Money,
        gastos: Money,
        capacidad_total: u32,
        entradas_emitidas: u32,
        aforo_proyectado: u32,
        metodos_pago: PaymentMethodBreakdown,
        categorias_gasto: ExpenseCategoryBreakdown,
    ) -> Self {
        let total_proyectado = cobrado.add(&pendiente);
        let ganancia_neta = cobrado.sub_signed(&gastos);
        let ganancia_neta_soles = ganancia_neta as f64 / 100.0;

        Self {
            cobrado,
            pendiente,
            total_proyectado,
            gastos,
            ganancia_neta,
            ganancia_neta_soles,
            capacidad_total,
            entradas_emitidas,
            aforo_proyectado,
            metodos_pago,
            categorias_gasto,
        }
    }
}

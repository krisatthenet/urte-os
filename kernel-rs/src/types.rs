//! Base types for the URTE kernel — Rust rewrite of `include/urte/types.h`.
//!
//! Derived from the `urtecore` Capella MBSE model (see `model/urtecore.model.json`).
//! Scale numbering matches the POSIX Kernel Technical Specification: 0 = Molecular
//! .. 6 = Planetary.

use std::fmt;

/// Multi-scale level (model: Operational Entity hierarchy Ecosystem -> .. -> AI core).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ScaleLevel {
    Molecular = 0,
    Cellular = 1,
    Tissue = 2,
    Organ = 3,
    Ecosystem = 4,
    Biome = 5,
    Planetary = 6,
}

impl ScaleLevel {
    pub const ALL: [ScaleLevel; 7] = [
        ScaleLevel::Molecular,
        ScaleLevel::Cellular,
        ScaleLevel::Tissue,
        ScaleLevel::Organ,
        ScaleLevel::Ecosystem,
        ScaleLevel::Biome,
        ScaleLevel::Planetary,
    ];

    /// Parse from the lowercase identifier used in the pipeline DSL.
    pub fn from_ident(s: &str) -> Option<ScaleLevel> {
        Some(match s {
            "molecular" => ScaleLevel::Molecular,
            "cellular" => ScaleLevel::Cellular,
            "tissue" => ScaleLevel::Tissue,
            "organ" => ScaleLevel::Organ,
            "ecosystem" => ScaleLevel::Ecosystem,
            "biome" => ScaleLevel::Biome,
            "planetary" => ScaleLevel::Planetary,
            _ => return None,
        })
    }
}

impl fmt::Display for ScaleLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ScaleLevel::Molecular => "MOLECULAR",
            ScaleLevel::Cellular => "CELLULAR",
            ScaleLevel::Tissue => "TISSUE",
            ScaleLevel::Organ => "ORGAN",
            ScaleLevel::Ecosystem => "ECOSYSTEM",
            ScaleLevel::Biome => "BIOME",
            ScaleLevel::Planetary => "PLANETARY",
        };
        f.write_str(s)
    }
}

/// Technology Readiness Level (1..9, ISO 16290) used by TRL-Pull scheduling.
pub type Trl = u8;

/// Guardrail decision (spec ch. 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
    EthicsVeto,
    PauseRequired,
}

/// Functional stages of the model's operational pipeline (OA activities),
/// in execution order (Sensing first, mitigation last).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    // Operational activity pipeline (Sensing -> therapy delivery mitigation).
    Sensing,
    DataGathering,
    Memorization,
    StringComparison,
    OperationalStatusCheck,
    HypothesisStatement,
    MeasureDefinition,
    MeasureGathering,
    MeasureCompose,
    MeasureSelectorCheck,
    ReactorTherapyAssumption,
    TherapyDeliveryMitigation,
    // Fabrication / terraforming flow (Sensing -> Dissemination).
    Crawling,
    Selection,
    Fabrication,
    Assembly,
    Dissemination,
}

impl Stage {
    /// Operational activity pipeline, in execution order.
    pub const PIPELINE: [Stage; 12] = [
        Stage::Sensing,
        Stage::DataGathering,
        Stage::Memorization,
        Stage::StringComparison,
        Stage::OperationalStatusCheck,
        Stage::HypothesisStatement,
        Stage::MeasureDefinition,
        Stage::MeasureGathering,
        Stage::MeasureCompose,
        Stage::MeasureSelectorCheck,
        Stage::ReactorTherapyAssumption,
        Stage::TherapyDeliveryMitigation,
    ];

    /// Fabrication / terraforming pipeline (model: `fabricationPipeline`).
    pub const FABRICATION: [Stage; 6] = [
        Stage::Sensing,
        Stage::Crawling,
        Stage::Selection,
        Stage::Fabrication,
        Stage::Assembly,
        Stage::Dissemination,
    ];

    /// Parse from the snake_case identifier used in the pipeline DSL.
    pub fn from_ident(s: &str) -> Option<Stage> {
        Some(match s {
            "sensing" => Stage::Sensing,
            "data_gathering" => Stage::DataGathering,
            "memorization" => Stage::Memorization,
            "string_comparison" => Stage::StringComparison,
            "operational_status_check" => Stage::OperationalStatusCheck,
            "hypothesis_statement" => Stage::HypothesisStatement,
            "measure_definition" => Stage::MeasureDefinition,
            "measure_gathering" => Stage::MeasureGathering,
            "measure_compose" => Stage::MeasureCompose,
            "measure_selector_check" => Stage::MeasureSelectorCheck,
            "reactor_therapy_assumption" => Stage::ReactorTherapyAssumption,
            "therapy_delivery_mitigation" => Stage::TherapyDeliveryMitigation,
            "crawling" => Stage::Crawling,
            "selection" => Stage::Selection,
            "fabrication" => Stage::Fabrication,
            "assembly" => Stage::Assembly,
            "dissemination" => Stage::Dissemination,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Stage::Sensing => "Sensing",
            Stage::DataGathering => "Data gathering",
            Stage::Memorization => "Memorization",
            Stage::StringComparison => "string comparison",
            Stage::OperationalStatusCheck => "operational status check",
            Stage::HypothesisStatement => "hypothesis statement",
            Stage::MeasureDefinition => "measure definition",
            Stage::MeasureGathering => "measure gathering",
            Stage::MeasureCompose => "measure compose",
            Stage::MeasureSelectorCheck => "measure selector check",
            Stage::ReactorTherapyAssumption => "reactor therapy assumption",
            Stage::TherapyDeliveryMitigation => "therapy delivery mitigation",
            Stage::Crawling => "Crawling",
            Stage::Selection => "Selection",
            Stage::Fabrication => "Fabrication",
            Stage::Assembly => "Assembly",
            Stage::Dissemination => "Dissemination",
        }
    }
}

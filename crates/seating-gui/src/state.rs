use seating_core::{
    ClosenessRule, OptimizationResult, Person, TableShape, TableTypeConfig, ValidationReport,
};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) enum Msg {
    SelectTab(Tab),
    FocusNext,
    FocusPrevious,
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    ExportProjectCsv,
    ImportPeople,
    SavePeople,
    ExportPeople,
    ImportCloseness,
    SaveCloseness,
    ExportCloseness,
    ImportTables,
    SaveTables,
    ExportTables,
    AddPerson,
    DeletePerson(usize),
    UpdatePersonId(usize, String),
    UpdatePersonName(usize, String),
    UpdatePersonTableType(usize, MaybeStringChoice),
    UpdatePersonLockedTable(usize, MaybeUsizeChoice),
    UpdatePersonLockedSeat(usize, MaybeUsizeChoice),
    UpdateNewGroup(usize, String),
    AddGroup(usize),
    RemoveGroup(usize, usize),
    AddClosenessRule,
    DeleteClosenessRule(usize),
    UpdateClosenessLeft(usize, String),
    UpdateClosenessRight(usize, String),
    SelectClosenessLeft(usize, String),
    SelectClosenessRight(usize, String),
    UpdateClosenessScore(usize, String),
    AddTableConfig,
    DeleteTableConfig(usize),
    UpdateTableTypeId(usize, String),
    UpdateTableShape(usize, ShapeChoice),
    UpdateMaxPeople(usize, String),
    UpdateMinPeople(usize, String),
    UpdateRecommendedPeople(usize, String),
    UpdateNumberOfTables(usize, String),
    UpdatePeoplePerSide(usize, String),
    SeedChanged(String),
    AttemptsChanged(String),
    IterationsChanged(String),
    SolutionsChanged(String),
    ProximityWeightChanged(String),
    UsedTableWeightChanged(String),
    OptimalTableSizeWeightChanged(String),
    Optimize,
    OptimizeFinished(Result<OptimizationResult, ValidationReport>),
    SaveSeating,
    ExportPlanSvg,
    ExportPlanPng,
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

/// A whole-project action that would discard unsaved work, awaiting a second
/// confirming activation of the same message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingConfirm {
    NewProject,
    OpenProject,
    ExportCsv(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    People,
    Closeness,
    Tables,
    Optimize,
    SeatingPlan,
    Diagnostics,
}

impl Tab {
    pub(crate) const ALL: [Tab; 6] = [
        Tab::People,
        Tab::Closeness,
        Tab::Tables,
        Tab::Optimize,
        Tab::SeatingPlan,
        Tab::Diagnostics,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Tab::People => "People",
            Tab::Closeness => "Closeness",
            Tab::Tables => "Tables",
            Tab::Optimize => "Optimize",
            Tab::SeatingPlan => "Seating Plan",
            Tab::Diagnostics => "Validation / Diagnostics",
        }
    }
}

/// Severity of [`GuiApp`](crate::app::GuiApp)'s single message-bar line, used
/// to pick a visually distinct container style (info/success/error).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MessageKind {
    #[default]
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MaybeStringChoice {
    pub(crate) label: String,
    pub(crate) value: Option<String>,
}

impl Display for MaybeStringChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MaybeUsizeChoice {
    pub(crate) label: String,
    pub(crate) value: Option<usize>,
}

impl Display for MaybeUsizeChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeChoice {
    Round,
    Rectangular,
    Square,
}

impl ShapeChoice {
    pub(crate) const ALL: [ShapeChoice; 3] = [
        ShapeChoice::Round,
        ShapeChoice::Rectangular,
        ShapeChoice::Square,
    ];
}

impl Display for ShapeChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ShapeChoice::Round => "round",
            ShapeChoice::Rectangular => "rectangular",
            ShapeChoice::Square => "square",
        };
        write!(f, "{label}")
    }
}

impl From<TableShape> for ShapeChoice {
    fn from(value: TableShape) -> Self {
        match value {
            TableShape::Round => ShapeChoice::Round,
            TableShape::Rectangular => ShapeChoice::Rectangular,
            TableShape::Square => ShapeChoice::Square,
        }
    }
}

impl From<ShapeChoice> for TableShape {
    fn from(value: ShapeChoice) -> Self {
        match value {
            ShapeChoice::Round => TableShape::Round,
            ShapeChoice::Rectangular => TableShape::Rectangular,
            ShapeChoice::Square => TableShape::Square,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PersonRowState {
    pub(crate) person: Person,
    pub(crate) new_group: String,
}

impl Default for PersonRowState {
    fn default() -> Self {
        Self {
            person: Person {
                id: String::new(),
                name: String::new(),
                table_type: None,
                groups: Vec::new(),
                locked_table: None,
                locked_seat: None,
            },
            new_group: String::new(),
        }
    }
}

impl From<Person> for PersonRowState {
    fn from(person: Person) -> Self {
        Self {
            person,
            new_group: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClosenessRowState {
    pub(crate) left_id: String,
    pub(crate) right_id: String,
    pub(crate) score_input: String,
}

impl Default for ClosenessRowState {
    fn default() -> Self {
        Self {
            left_id: String::new(),
            right_id: String::new(),
            score_input: "0".to_string(),
        }
    }
}

impl From<ClosenessRule> for ClosenessRowState {
    fn from(rule: ClosenessRule) -> Self {
        Self {
            left_id: rule.left_id,
            right_id: rule.right_id,
            score_input: rule.score.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TableConfigState {
    pub(crate) table_type_id: String,
    pub(crate) shape: ShapeChoice,
    pub(crate) max_people_input: String,
    pub(crate) min_people_input: String,
    pub(crate) recommended_people_input: String,
    pub(crate) number_of_tables_input: String,
    pub(crate) people_per_side_input: String,
}

impl Default for TableConfigState {
    fn default() -> Self {
        Self {
            table_type_id: String::new(),
            shape: ShapeChoice::Round,
            max_people_input: "8".to_string(),
            min_people_input: String::new(),
            recommended_people_input: String::new(),
            number_of_tables_input: "1".to_string(),
            people_per_side_input: String::new(),
        }
    }
}

impl TableConfigState {
    pub(crate) fn from_pair(table_type_id: String, config: TableTypeConfig) -> Self {
        let people_per_side_input = config
            .people_per_side
            .as_ref()
            .map(|values| {
                values
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .unwrap_or_default();
        Self {
            table_type_id,
            shape: ShapeChoice::from(config.shape),
            max_people_input: config.max_people.to_string(),
            min_people_input: config
                .min_people
                .map(|value| value.to_string())
                .unwrap_or_default(),
            recommended_people_input: config
                .recommended_people
                .map(|value| value.to_string())
                .unwrap_or_default(),
            number_of_tables_input: config
                .number_of_tables
                .map(|value| value.to_string())
                .unwrap_or_default(),
            people_per_side_input,
        }
    }
}

pub(crate) fn selected_string_choice(
    options: &[MaybeStringChoice],
    value: &Option<String>,
) -> Option<MaybeStringChoice> {
    options
        .iter()
        .find(|choice| &choice.value == value)
        .cloned()
}

pub(crate) fn selected_usize_choice(
    options: &[MaybeUsizeChoice],
    value: Option<usize>,
) -> Option<MaybeUsizeChoice> {
    options.iter().find(|choice| choice.value == value).cloned()
}

pub(crate) fn display_path(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(not saved yet)".to_string())
}

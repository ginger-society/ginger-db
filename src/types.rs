// Database.toml related start

use serde::{Deserialize, Serialize};

// Ginger models generator structs starts
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OnDeleteOptions {
    Cascade,
    Protect,
    SetNull,
    DoNothing,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum ColumnType {
    CharField,
    BooleanField,
    DateField,
    DateTimeField,
    ForeignKey,
    BigAutoField,
    PositiveIntegerField,
    FloatField,
    ManyToManyField,
    TextField,
    OneToOneField,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Table,
    Enum,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Schema {
    pub id: String,
    pub rows: Vec<Row>,
    pub data: Data,
    #[serde(rename = "type")]
    pub schema_type: SchemaType,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub data: FieldData,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct WatchContent{
    pub resource_id: String,
    pub event: String
}


#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum DefaultValue {
    Boolean(bool),
    String(String),
}
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct FieldData {
    pub name: String,
    #[serde(rename = "field_name")]
    pub field_name: String,
    #[serde(rename = "type")]
    pub field_type: ColumnType,
    pub null: Option<bool>,
    pub options_target: Option<String>,
    pub default: Option<DefaultValue>,
    pub max_length: Option<String>,

    // Forign key related
    pub target: Option<String>,
    pub related_name: Option<String>,
    pub on_delete: Option<OnDeleteOptions>,
    pub auto_now_add: Option<bool>,
    pub auto_now: Option<bool>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ForeignKeyData {
    pub id: String,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Data {
    pub id: String,
    #[serde(rename = "table_name")]
    pub table_name: String,
    pub name: String,
    #[serde(default)]
    pub options: Option<Vec<OptionData>>,
    pub docs: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct OptionData {
    pub value: String,
    pub label: String,
}

// Ginger models generator structs ends

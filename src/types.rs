// Database.toml related start

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct DBSchema {
    pub url: String,
    pub lang: LANG,
    pub orm: ORM,
    pub root: String,
    pub schema_id: Option<String>,
    pub branch: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DBTables {
    pub names: Vec<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DBConfig {
    pub schema: DBSchema,
    pub tables: DBTables,
}

// Database.toml related structs ends

#[derive(Debug, Serialize, Deserialize)]
pub enum ORM {
    TypeORM,
    SQLAlchemy,
    DjangoORM,
    Diesel,
}

impl fmt::Display for ORM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ORM::TypeORM => write!(f, "TypeORM"),
            ORM::SQLAlchemy => write!(f, "SQLAlchemy"),
            ORM::DjangoORM => write!(f, "DjangoORM"),
            ORM::Diesel => write!(f, "Diesel"),
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub enum LANG {
    Rust,
    TS,
    Python,
}

impl LANG {
    pub fn all() -> Vec<LANG> {
        vec![LANG::Rust, LANG::TS, LANG::Python]
    }
}

impl fmt::Display for LANG {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LANG::Rust => write!(f, "Rust"),
            LANG::TS => write!(f, "TS"),
            LANG::Python => write!(f, "Python"),
        }
    }
}

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

use serde::Deserialize;
use serde_json::Result;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum OnDeleteOptions {
    Cascade,
    Protect,
    SetNull,
    DoNothing,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
enum ColumnType {
    CharField,
    BooleanField,
    DateField,
    ForeignKey,
    BigAutoField,
    PositiveIntegerField,
    FloatField,
    ManyToManyField,
    TextField,
    OneToOneField,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum SchemaType {
    Table,
    Enum,
}

#[derive(Deserialize, Debug)]
struct Schema {
    id: String,
    rows: Vec<Row>,
    data: Data,
    #[serde(rename = "type")]
    schema_type: SchemaType,
}

#[derive(Deserialize, Debug)]
struct Row {
    id: String,
    data: FieldData,
}

#[derive(Deserialize, Debug)]
struct FieldData {
    name: String,
    #[serde(rename = "field_name")]
    field_name: String,
    #[serde(rename = "type")]
    field_type: ColumnType,
    null: Option<bool>,
    options_target: Option<String>,

    // Forign key related
    target: Option<String>,
    related_name: Option<String>,
    on_delete: Option<OnDeleteOptions>,
}

#[derive(Deserialize, Debug)]
struct ForeignKeyData {
    id: String,
}

#[derive(Deserialize, Debug)]
struct Data {
    id: String,
    #[serde(rename = "table_name")]
    table_name: String,
    name: String,
    #[serde(default)]
    options: Option<Vec<OptionData>>,
}

#[derive(Deserialize, Debug)]
struct OptionData {
    value: String,
    label: String,
}

fn main() -> Result<()> {
    println!("Hello, world!");

    // Open the file in read-only mode with buffer.
    let file = File::open("src/db.design.json").unwrap();
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `Schema`.
    let schemas: Vec<Schema> = serde_json::from_reader(reader).unwrap();

    // Print the schemas to see if they are deserialized correctly.
    for schema in schemas {
        println!("{:#?}", schema);
    }

    Ok(())
}

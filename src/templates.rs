use tera::Tera;

pub const admin_py: &str = include_str!("../templates/admin.py.tpl");
pub const db_design_json: &str = include_str!("../templates/db.design.json.tpl");
pub const docker_compose_yml: &str = include_str!("../templates/docker-compose.yml.tpl");
pub const modes_py: &str = include_str!("../templates/models.py.tpl");

pub fn get_renderer() -> Tera {
    let mut tera = Tera::default();
    tera.add_raw_template("admin.py.tpl", admin_py).unwrap();
    tera.add_raw_template("db.design.json.tpl", db_design_json)
        .unwrap();
    tera.add_raw_template("docker-compose.yml.tpl", docker_compose_yml)
        .unwrap();
    tera.add_raw_template("models.py.tpl", modes_py).unwrap();
    return tera;
}

use tera::Tera;

pub const ADMIN_PY: &str = include_str!("../templates/admin.py.tpl");
pub const DOCKER_COMPOSE_YML: &str = include_str!("../templates/docker-compose.yml.tpl");
pub const MODES_PY: &str = include_str!("../templates/models.py.tpl");

pub fn get_renderer() -> Tera {
    let mut tera = Tera::default();
    tera.add_raw_template("admin.py.tpl", ADMIN_PY).unwrap();
    tera.add_raw_template("docker-compose.yml.tpl", DOCKER_COMPOSE_YML)
        .unwrap();
    tera.add_raw_template("models.py.tpl", MODES_PY).unwrap();
    return tera;
}

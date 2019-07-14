
#[derive(Debug, Clone)]
pub struct Message {
    pub from: Option<String>,
    pub to: Vec<String>,
    pub data: Vec<String>,
}


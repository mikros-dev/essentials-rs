use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Form {
    parts: Vec<(String, Part)>,
}

#[derive(Debug)]
pub enum Part {
    Text(String),
    Bytes {
        data: Vec<u8>,
        file_name: Option<String>,
        mime: Option<String>,
        headers: HashMap<String, String>,
    },
}

impl Form {
    #[must_use]
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    #[must_use]
    pub fn text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.parts.push((name.into(), Part::Text(value.into())));
        self
    }

    #[must_use]
    pub fn bytes(
        mut self,
        name: impl Into<String>,
        data: Vec<u8>,
        file_name: Option<String>,
        mime: Option<String>,
    ) -> Self {
        self.parts.push((
            name.into(),
            Part::Bytes {
                data,
                file_name,
                mime,
                headers: HashMap::new(),
            },
        ));
        self
    }
}

impl From<Form> for reqwest::multipart::Form {
    fn from(value: Form) -> Self {
        let mut out = reqwest::multipart::Form::new();

        for (name, part) in value.parts {
            out = match part {
                Part::Text(v) => out.text(name, v),
                Part::Bytes {
                    data,
                    file_name,
                    mime,
                    ..
                } => {
                    // Validate mime here.
                    let mime = mime.filter(|m| {
                        reqwest::multipart::Part::text(String::new())
                            .mime_str(m)
                            .is_ok()
                    });

                    let mut p = reqwest::multipart::Part::bytes(data);
                    if let Some(f) = file_name {
                        p = p.file_name(f);
                    }

                    if let Some(m) = mime {
                        p = p.mime_str(&m).expect("mime already validated");
                    }

                    out.part(name, p)
                }
            };
        }

        out
    }
}

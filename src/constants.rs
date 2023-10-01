pub const SNIPPEXT: &'static str = "snippext";

pub const DEFAULT_SNIPPEXT_CONFIG: &str = include_str!("./default_snippext_config.yaml");
pub const DEFAULT_START: &'static str = "snippet::start";
pub const DEFAULT_END: &'static str = "snippet::end";
pub const DEFAULT_TEMPLATE: &'static str = r#"```{{lang}}
{{snippet~}}
```
{{#unless omit_source_link}}
<a href='{{source_link}}' title='Snippet source file'>snippet source</a>
{{/unless}}
"#;
pub const DEFAULT_OUTPUT_FILE_EXTENSION: &'static str = "md";
pub const DEFAULT_SOURCE_FILES: &'static str = "**";
pub const DEFAULT_OUTPUT_DIR: &'static str = "./generated-snippets/";
pub const SNIPPEXT_TEMPLATE_ATTRIBUTE: &'static str = "template";
pub const DEFAULT_TEMPLATE_IDENTIFIER: &'static str = "default";
pub const DEFAULT_GIT_BRANCH: &'static str = "main";

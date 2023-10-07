pub const SNIPPEXT: &str = "snippext";

pub const DEFAULT_SNIPPEXT_CONFIG: &str = include_str!("./default_snippext_config.yaml");
pub const DEFAULT_START: &str = "snippet::start";
pub const DEFAULT_END: &str = "snippet::end";
pub const DEFAULT_TEMPLATE: &str = r#"```{{lang}}
{{snippet~}}
```
{{#unless omit_source_link}}
<a href='{{source_link}}' title='Snippet source file'>snippet source</a>
{{/unless}}
"#;
pub const DEFAULT_OUTPUT_FILE_EXTENSION: &str = "md";
pub const DEFAULT_SOURCE_FILES: &str = "**";
pub const DEFAULT_OUTPUT_DIR: &str = "./generated-snippets/";
pub const SNIPPEXT_TEMPLATE_ATTRIBUTE: &str = "template";
pub const DEFAULT_TEMPLATE_IDENTIFIER: &str = "default";
pub const DEFAULT_GIT_BRANCH: &str = "main";

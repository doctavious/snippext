# Snippext
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![CI](https://github.com/doctavious/snippext/workflows/CI/badge.svg)](https://github.com/doctavious/snippext/actions?query=workflow%3ACI)

Snippext extracts snippets from source files and merges them into your documentation.

> [!WARNING]
> Currently still working through kinks and there may be breaking changes in upcoming updates

## Install

Binary downloads of Snippext can be found on the [Releases page](https://github.com/doctavious/snippext/releases/latest).

You can also install Snippext via an installer script that will automatically grab the latest version of Snippext and install it locally.

You can fetch that script, and then execute it locally.

```shell 
curl -fsSL -o install_snippext.sh https://raw.githubusercontent.com/doctavious/snippext/main/install.sh
chmod 700 install_snippext.sh
./install_snippext.sh
```

You can also do

```bash 
curl https://raw.githubusercontent.com/doctavious/snippext/main/install.sh | bash
```

If you need to install a specific version for some reason you can pass the version as the first argument to the script.


## Configure

First thing you'll want to do is configure Snippext via a `snippext.yaml` configuration file. You an either create a `snippext.yaml` file at the root of your project or use the Snippext CLI `init` command which will present you with a series of prompts to configure the `snippext.yaml` configuration file for you. An example of a complete `snippext.yaml` file: 
<!-- snippext::start default_snippext_config {"omit_source_link": true } -->
```yaml
start: "snippet::start"
end: "snippet::end"
templates:
  default: |-
    ```{{lang}}
    {{snippet~}}
    ```
    {{#unless omit_source_link}}
    <a href='{{source_link}}' title='Snippet source file'>snippet source</a>
    {{/unless}}
  raw: "{{snippet}}"
sources:
# extract from local files
- type: Local
  files:
    - "**"

# extract from remote Git repo
#- type: Git
#  repository: https://github.com/doctavious/snippext.git
#  branch: main
#  cone_patterns:
#    - ./src/*.rs
#  files:
#    - "**"

# extract from URL
#- type:  http://localhost/hi

output_dir: "./generated-snippets/"
output_extension: "md"  # Extension for generated files written to the output directory
# targets: ./docs  # List of glob patters, separated by spaces, that contain the files to be spliced with the code snippets.
# link_format: GitHub  # Defines the format of snippet source links that appear under each snippet.
omit_source_links: false
missing_snippets_behavior: Warn
retain_nested_snippet_comments: false
enable_autodetect_language: true
selected_lines_include_ellipses: false
```
<!-- snippext::end -->

## Defining Snippets

Use comments that begin with a Snippext prefix to identify code snippets in source files and the locations where they should be merged into target files.

## Source Files

The first thing we need to do is define snippets in your source code. Wrap the code snippets in a comment that starts with the Snippext `start` prefix, default value is `snippet::start` followed by a unique snippet identifier. End the snippet with a comment that starts with the Snippext `end` prefix, default value is `snippet::end`. It should look something similar to this:

<!-- snippext::start readme_example {"omit_source_link": true } -->
```rust
// snippet::start rust_main
fn main() {
    println!("Hello, Snippext!");
}
// snippet::end
```
<!-- snippext::end -->

In the snippet above, "rust_main" is the unique identifier for the code snippet. While identifiers can be any string which doesn't contain whitespace characters, which allows us to support including [URL](#including-snippet-from-url) and [file](#including-snippet-from-file), its generallly recommended for identifiers to contain only letters, numbers, hyphens, and underscores.

> [!NOTE]
> Named C# regions will also be picked up, with the name of the region used as the identifier.

### Source Features

#### Retain Nested Snippet Comments

Snippets can be nested in other snippets. By default, nested snippet comments are omitted from being included in the parent snippet content. Nested snippet comments can be retained by globally by either passing the `--retain-nested-snippet-comments` flag to the `extract` CLI command or setting it to true within the snippet configuration file. You can also enable it on individual snippets by including it in the JSON configuration of the source snippet.

## Target Files

Next, we need to identify places in target files where we want to insert snippets into. Similar to source files, we wrap the location with a comment that references the identifier of the code snippet that will be inserted there:

```
<!-- snippet::start rust_main -->
<!-- snippet::end -->
```

In the example above, the "readme_example" code snippet will be spliced between the comments. Any text inside the comment will be replaced by the code snippet. This allows for a default snippet to be included if for any reason the referenced identifier was not extracted from the source.

When rendering snippet content, Snippext will remove leading spaces from indented code snippets.

### Target Features

To customize how a snippet is rendered add JSON configuration after the identifier of the snippet start line. An example would look like

<!-- snippext::start readme_attributes_example {"omit_source_link": true } -->
```rust
// snippet::start snippet_with_attributes {"template": "raw" }
fn main() {
    println!("Hello, Snippext!");
}
// snippet::end
```
<!-- snippext::end -->

#### Template

The `template` attribute specifies the template that will be used to render the snippet. If not specified the default template will be used. 

See [Custom Template](#custom-template)

#### Omit Source Link

The `omit_source_link` attribute determines whether source links should be included in the rendering of the snippet. If not specified it will default  to false.

#### Select Specific Lines

You can use a source snippet in multiple places, so you may wish to customize which lines are rendered in each location. Add a `selected_lines` configuration to the JSON configuration.

> [!NOTE]
> Nested comments don't count to line numbers unless you've enabled the flag to retain them in source content

If you would like to include ellipses comments, e.g. `// ...`,  for any gaps when using `selected_lines` you can enable `selected_lines_include_ellipses`

### Including Snippet From URL

Snippets that start with `http` will be downloaded and the contents rendered. For example:

```
<!-- snippet::start https://raw.githubusercontent.com/doctavious/snippext/main/LICENSE -->
<!-- snippet::end -->
```

URL contents are downloaded to `temp/snippext`

### Including Snippet From File

If no snippet is found matching the identifier Snippext will treat it as a file and the contents rendered. For example:


<!-- snippext::start LICENSE { "selected_lines": ["1"], "selected_lines_include_ellipses": true } -->
```
MIT License
...
```
<!-- snippext::end -->

## Advanced

#### Custom Templates

Snippext uses [handlebar](https://handlebarsjs.com/) templates to produce the generated snippets. Default templates are provided. To customize a snippet's content you can provide your own templates.

- snippet - Content of the source snippet
- source_path - Path to the source file that contains the snippet
- source_link_prefix - Source link prefix based on link format if provided
- source_link - Full source link based on link format
- omit_source_link - Flag that determines whether source links should be included in rendered output.
- selected_lines - The selected lines, if present, from the target snippet attributes
- selected_lines_include_ellipses - Determines whether ellipsis should be added to gaps when `select_lines` attribute is used to render snippets.

You can provide extra information by adding configuration attributes on source and target snippets. Any additional attributes are made available during the template rendering process. Combined with a custom template, this makes it possible to include extra information in a generated snippet.

## Clear Snippets

To remove snippet contents, keeping the snippext comment intact, from target files use the `clear` command.

```bash
snippext clear
```

This will use configuration from your `snippext.yaml` if present otherwise it will use default configuration shown above. You can also pass in CLI args to configure.

If you prefer to remove the entire snippet, including the snippet comment, provide the `--delete` flag.

```bash
snippext clear --delete
```
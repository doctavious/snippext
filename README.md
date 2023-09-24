# Snippext
[![crates.io](https://img.shields.io/crates/v/snippext.svg)](https://crates.io/crates/snippext)
[![Released API docs](https://docs.rs/snippext/badge.svg)](https://docs.rs/snippext)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![CI](https://github.com/doctavious/snippext/workflows/CI/badge.svg)](https://github.com/doctavious/snippext/actions?query=workflow%3ACI)

Snippext extracts snippets from source files and merges them into your documentation.

## Install

TODO: provide instructions

## Configuration

TODO: 
- provide default config
- explanation of fields


## Defining Snippets

Use comments that begin with a Snippext prefix to identify code snippets in source files and the locations where they should be merged into target files.

## Source Files

The first thing we need to do is define snippets in your source code. Wrap the code snippets in a comment that starts with the Snippext `start` prefix, default value is `snippet::start` followed by a unique snippet identifier. End the snippet with a comment that starts with the Snippext `end` prefix, default value is `snippet::end`. It should look something similar to this:

<!-- snippext::start readme_example {"omit_source_link": true } -->
```rust
// snippext::start rust_main
fn main() {
    println!("Hello, Snippext!");
}
// snippext::end
```
<!-- snippext::end -->

[//]: # (TODO: mentioned id Unique identifiers can contain letters, numbers, hyphens, and underscores.)

[//]: # (TODO: mention The code snippets will do smart trimming of snippet indentation. remove leading spaces from indented code snippets.)


> [!NOTE]  
> Named C# regions will also be picked up, with the name of the region used as the identifier.

### Features

#### Retain Nested Snippet Comments

Snippets can be nested in other snippets. By default, nested snippet comments are omitted from being included in the parent snippet content. Nested snippet comments can be retained by globally by either passing the `retain_nested_snippet_comments` flag to the `extract` CLI command or setting it to true within the snippet configuration file. You can also enable it on individual snippets by including it in the JSON configuration of the source snippet.

## Target Files

Next, we need to identify places in target files where we want to insert snippets into. Similar to source files, we wrap the location with a comment that references the identifier of the code snippet that will be inserted there:

```
<!-- snippet::start readme_example -->
<!-- snippet::end -->
```

In the example above, the "readme_example" code snippet will be spliced between the comments. Any text inside the comment will be replaced by the code snippet. This allows for a default snippet to be included if for any reason the referenced identifier was not extracted from the source.

### Features

To customize how a snippet is rendered add JSON configuration after the identifier of the snippet start line. An example would look like

<!-- snippext::start readme_attributes_example {"omit_source_link": true } -->
```rust
// snippext::start snippet_with_attributes {"template": "raw" }
fn main() {
    println!("Hello, Snippext!");
}
// snippext::end
```
<!-- snippext::end -->

#### Template

The `template` attribute specifies the template that will be used to render the snippet. If not specified the default template will be used. 

See [Custom Template](#custom-template)

#### Omit Source Link

The `omit_source_link` attribute determines whether source links should be included in the rendering of the snippet. If not specified it will default  to false.

#### Select Specific Lines

You can use a source snippet in multiple places, so you may wish to customize which lines are rendered in each location. Add a `selected_lines` configuration to the JSON configuration.

### Including Snippet From URL

Snippets that start with `http` will be downloaded and the contents rendered. For example:

```
<!-- snippet::start https://raw.githubusercontent.com/doctavious/snippext/main/LICENSE -->
<!-- snippet::end -->
```

URL contents are downloaded to `temp/snippext`

### Including Snippet From File

If no snippet is found matching the identifier Snippext will treat it as a file and the contents rendered. For example:

``` 
<!-- snippet::start LICENSE -->
<!-- snippet::end -->
```

## Advanced

#### Custom Template


## Clear Snippets

To remove snippets from target files
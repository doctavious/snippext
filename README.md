# snippext

Extract snippets from files.

## Snippets

Snippets are useful when you want to identify specific regions of a file to extract and include the context in other files. 
Extracted snippets will be output to specified output directory keeping the directory layout.

Snippets have the following start/end formats

```
start: <comment> <begin flag><identifier>[<comma separated key=value pairs>]
end: <comment> <end flag>
```

Example
```
// snippet::main[lang=rust] // (1) (2) (3)
fn main() {

    // snippet::nested // (5)
    println!("printing...")
    // end::nested
}
// end::main // (4)
```
1. To indicate the start of a snippet, insert a comment line in the code.
2. Assign an identifier to the snippet directive. In this example, the tag is named main. The snippet identifier will be sanitized
3. Within the brackets `[]` you can include a comma separated list of key/value pairs which are called `attributes` and can be used  in custom templates. 
4. Insert another comment line where you want the snippet to end.
5. You can also include nested snippets. The nested snippet comment will not be included in the extracted output.

**Important**
The snippet::[] and end::[] directives should be placed after a line comment as defined by the language of the source file. 

Assuming that the above example lives in `src/main.rs` two files will be created
1. src/main.rs/main.md
2. src/main.rs/nested.md

### Advanced

#### Custom Template

snippext by default writes out the content of the snippet within the boundaries of the snippet/end directives. 
You can alter the output by providing a custom template.

Taking the example above lets say you want to add a fenced code block that included the source language. You could set the template to the following
```
"```{{lang}}\n{{snippet}}\n```\n",
```

which would produce the following

    ```rust
    fn main() {

        println!("printing...")
    }
    ```

Extract snippets from relative files into output_dir. It is not mandatory to terminate a snippet, the extractor will simply add line until EOF. When a directory is passed as an argument, all files from directory will be parsed.

## CLI Usage

### Command Line Arguments

```
snippext [FLAGS] [OPTIONS] [sources]
```

**Flags:**
```
-h, --help          Prints help information
-V, --version       Prints version information
```

**Options:**
```
-b, --begin <begin>                      flag to mark beginning of a snippet [default: snippet::]
-c, --comment-prefix <comment-prefix>     [default: // ]
-e, --end <end>                          flag to mark ending of a snippet [default: end::]
-x, --extension <extension>              extension for generated files [default: .md]
-o, --output-dir <output-dir>            directory in which the files will be generated [default: ./snippets/]
```

**Args:**

```
<sources> space delimited list of files or directories [./file|./directory/]
```

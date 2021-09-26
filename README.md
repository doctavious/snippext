# snippext

Extract snippets from relative files into output_dir. It is not mandatory to terminate a snippet, the extractor will simply add line until EOF. When a directory is passed as an argument, all files from directory will be parsed.


The tag::[] and end::[] directives should be placed after a line comment as defined by the language of the source file.

format is 
```
<comment> <prefix><identifier>[<comma separated key=value pairs>]
```

Arguments

[./file|./directory/]

flags 

. --begin
. --end 
. --output_dir

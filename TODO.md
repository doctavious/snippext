Do we also want to be able to write snippets to some file that doesnt natively support includes?

maybe just leverage something like one of the following

M4 https://stackoverflow.com/questions/4779582/markdown-and-including-multiple-files/36104553#36104553

https://jekyllrb.com/docs/includes/


Issue I have with M4 is that you need to keep an input file and a result/output file.
However, this doesn't look to bad https://github.com/andreasbm/readme

what about using the same concept of snippets but instead of you write in between the comments.
This would be primarily useful for README.md files

so if you have a markdown file like 


some content 

 <!-- snippext_start::name --><!-- snippext_end::name -->


See https://github.com/temporalio/snipsync


ability to clear snippets from output directory as well as target files


support source files from git repo
owner,
repo,
ref,
do sparse checkout based on sources glob


review https://github.com/crate-ci/cargo-release/tree/e19c222ae9010c6ab1ef8aeb5080ab2d766764d1/src

just shell out to git instead of use git2-rs?

impl Default 


target 

asciidoc uses the following  
```
include::version@component:module:partial$name-of-file.adoc[optional attributes]

or more specifically 

version@component:module:family$relative/path-to/resource.ext
```
for snippext lets use 

`snippext:include`

because I dont want to users to always have to specify the relative what if we did

```
snippext::include:<identifier>

or use fully qualified identifier if there is a conflict with identifier
snippext:include::<path to resource>:<identifier>
```


multiple templates that users can then specify via an attribute?
Ex
``` 
// snippet::main[lang=rust,snippext_template=lang_template]
fn main() {

    // snippet::nested
    println!("printing...")
    // end::nested
}
// end::main
```
which we will use when we generate. For targets use the following

``` 
<!-- snippet::main[snippext_template=code] -->
<!-- end::main -->
```

add option (snippext_source=true) to include link to source prior to snippet
``` 
`[${path}](${url})`
```
which we can add as an attribute
Maybe we always add source attribute and force you to provide via custom template?
Would need a way to support github/gitlab/bitbucket/gitea.io/etc


add init command that will create a default config or allow user to specify


use `basic` or `default` as the default snippet identifier?

add path and file name as attributes that can be included in snippet but how to handle github/gitlab/bitbucket
parse out owner and repo from url. add "blob" and then the ref (commit)
what to do for local files? maybe see if git remote returns anything if yes then parse and if not use local path?





- targets
  - target: "file"
  - target: "another_file"
    output: "another_file_update"



review structure of https://github.com/rust-lang/mdBook/blob/master/src/lib.rs

switch over to https://github.com/Lutetium-Vanadium/requestty which has a way to do tests https://github.com/Lutetium-Vanadium/requestty/blob/master/tests/helpers/mod.rs


https://github.com/SimonCropp/MarkdownSnippets - has decent version of link to source
- https://github.com/SimonCropp/MarkdownSnippets
- https://github.com/SimonCropp/MarkdownSnippets
- this also allows to include full file
- this also allows to include from url


snipsync has a flag `enable_code_dedenting` to toggle trimming leading spaces

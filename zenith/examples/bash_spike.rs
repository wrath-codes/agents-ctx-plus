use ast_grep_language::{SupportLang, LanguageExt};

fn dump_tree(source: &str) {
    let root = SupportLang::Bash.ast_grep(source);
    fn walk(node: &ast_grep_core::Node<impl ast_grep_core::Doc>, depth: usize) {
        let indent = "  ".repeat(depth);
        let kind = node.kind();
        let text = node.text().to_string();
        let short = if text.len() > 80 { format!("{}...", &text[..77]) } else { text };
        let short = short.replace('\n', "\\n");
        println!("{indent}{}: `{short}`", kind.as_ref());
        let children: Vec<_> = node.children().collect();
        for child in &children {
            walk(child, depth + 1);
        }
    }
    walk(&root.root(), 0);
}

fn main() {
    let source = r#"#!/bin/bash
# This is a doc comment for greet
# It has multiple lines
greet() {
    echo "Hello, $1!"
}

# Function using function keyword
function cleanup {
    rm -rf /tmp/myapp_*
}

# Variable assignments
FOO="bar"
export API_KEY="secret"
readonly MAX_RETRIES=3
local count=0
declare -x EXPORTED_VAR="value"
declare -i INTEGER_VAR=42

# Alias
alias ll='ls -la'
alias gs='git status'

# Export
export PATH="/usr/local/bin:$PATH"
export -f greet

# Conditional
if [ -f "$1" ]; then
    echo "File exists"
elif [ -d "$1" ]; then
    echo "Directory exists"
else
    echo "Not found"
fi

# Case
case "$1" in
    start)
        echo "Starting..."
        ;;
    stop)
        echo "Stopping..."
        ;;
    *)
        echo "Usage: $0 {start|stop}"
        ;;
esac

# For loop
for i in 1 2 3 4 5; do
    echo "$i"
done

# While loop
while read -r line; do
    echo "$line"
done < input.txt

# Until loop
until [ "$count" -ge 5 ]; do
    ((count++))
done

# Array declarations
declare -a MY_ARRAY=(one two three)
declare -A ASSOC_ARRAY=([key1]=val1 [key2]=val2)

# Heredoc
cat <<EOF
Hello World
This is a heredoc
EOF

cat <<-INDENTED
	indented heredoc
INDENTED

# Subshell
(cd /tmp && ls)

# Command group
{ echo "grouped"; echo "commands"; }

# Pipeline
ls -la | grep ".rs" | wc -l

# Command substitution
RESULT=$(date +%Y-%m-%d)
OLD_STYLE=`hostname`

# Trap
trap 'echo "Caught signal"; cleanup' EXIT INT TERM

# Source
source ./utils.sh
. ./helpers.sh

# Select
select opt in "Option1" "Option2" "Quit"; do
    case $opt in
        "Quit") break;;
        *) echo "Selected: $opt";;
    esac
done

# Here string
grep "pattern" <<< "$input_var"
"#;

    dump_tree(source);
}

use std::io::Write;

use liquid_error::{Result, ResultLiquidExt};

use compiler::tokenize;
use compiler::LiquidOptions;
use compiler::Token;
use compiler::{parse, unexpected_token_error};
use interpreter::Context;
use interpreter::Renderable;
use interpreter::Template;

#[derive(Debug)]
struct Include {
    name: String,
    partial: Template,
}

impl Renderable for Include {
    fn render_to(&self, writer: &mut Write, mut context: &mut Context) -> Result<()> {
        self.partial
            .render_to(writer, &mut context)
            .trace_with(|| format!("{{% include {} %}}", self.name).into())?;

        Ok(())
    }
}

fn parse_partial(name: &str, options: &LiquidOptions) -> Result<Template> {
    let content = options.include_source.include(name)?;

    let tokens = tokenize(&content)?;
    parse(&tokens, options).map(Template::new)
}

pub fn include_tag(
    _tag_name: &str,
    arguments: &[Token],
    options: &LiquidOptions,
) -> Result<Box<Renderable>> {
    let mut args = arguments.iter();

    let name = match args.next() {
        Some(&Token::StringLiteral(ref name)) => name,
        Some(&Token::Identifier(ref s)) => s,
        arg => return Err(unexpected_token_error("string", arg)),
    };

    let partial =
        parse_partial(name, options).trace_with(|| format!("{{% include {} %}}", name).into())?;

    Ok(Box::new(Include {
        name: name.to_owned(),
        partial,
    }))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::iter::FromIterator;
    use std::path;
    use std::sync;

    use compiler;
    use filters;
    use interpreter;
    use interpreter::ContextBuilder;
    use tags;
    use value;

    use super::*;

    fn options() -> LiquidOptions {
        let include_path = path::PathBuf::from_iter("tests/fixtures/input".split('/'));

        let mut options = LiquidOptions::default();
        options.include_source = Box::new(compiler::FilesystemInclude::new(include_path));
        options
            .tags
            .insert("include", (include_tag as compiler::FnParseTag).into());
        options.blocks.insert(
            "comment",
            (tags::comment_block as compiler::FnParseBlock).into(),
        );
        options
            .blocks
            .insert("if", (tags::if_block as compiler::FnParseBlock).into());
        options
    }

    #[test]
    fn include_tag_quotes() {
        let text = "{% include 'example.txt' %}";
        let tokens = compiler::tokenize(&text).unwrap();
        let template = compiler::parse(&tokens, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut filters: HashMap<&'static str, interpreter::BoxedValueFilter> = HashMap::new();
        filters.insert("size", (filters::size as interpreter::FnFilterValue).into());
        let mut context = ContextBuilder::new()
            .set_filters(&sync::Arc::new(filters))
            .build();
        context
            .stack_mut()
            .set_global_val("num", value::Value::scalar(5f64));
        context
            .stack_mut()
            .set_global_val("numTwo", value::Value::scalar(10f64));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "5 wat wot\n");
    }

    #[test]
    fn include_non_string() {
        let text = "{% include example.txt %}";
        let tokens = compiler::tokenize(&text).unwrap();
        let template = compiler::parse(&tokens, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut filters: HashMap<&'static str, interpreter::BoxedValueFilter> = HashMap::new();
        filters.insert("size", (filters::size as interpreter::FnFilterValue).into());
        let mut context = ContextBuilder::new()
            .set_filters(&sync::Arc::new(filters))
            .build();
        context
            .stack_mut()
            .set_global_val("num", value::Value::scalar(5f64));
        context
            .stack_mut()
            .set_global_val("numTwo", value::Value::scalar(10f64));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "5 wat wot\n");
    }

    #[test]
    fn no_file() {
        let text = "{% include 'file_does_not_exist.liquid' %}";
        let tokens = compiler::tokenize(&text).unwrap();
        let template = compiler::parse(&tokens, &options()).map(interpreter::Template::new);

        assert!(template.is_err());
        if let Err(val) = template {
            let val = val.to_string();
            println!("val={}", val);
            assert!(val.contains("Snippet does not exist"));
        }
    }
}

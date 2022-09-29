use {
    anyhow::{bail, Result},
    clap::Parser,
    convert_case::{Case, Casing},
    handlebars::Handlebars,
    log::*,
    pulldown_cmark::{html, Options, Parser as Markdown},
    serde::{Deserialize, Serialize},
    std::{
        fs,
        path::{Path, PathBuf},
    },
};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    #[clap(value_parser)]
    #[clap(value_name = "DIR")]
    #[clap(help = "path to input directory root")]
    input: PathBuf,

    #[clap(short, long, value_parser)]
    #[clap(value_name = "DIR")]
    #[clap(default_value = "_dist")]
    #[clap(help = "path to build output directory")]
    output: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct FrontMatter {
    title: String,
    tags: Option<Vec<String>>,
    datetime: Option<chrono::DateTime<chrono::FixedOffset>>,
    template: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Post {
    front_matter: FrontMatter,
    body: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    debug!("{:?}", cli);

    let templates_path = cli.input.join("templates");
    if !templates_path.exists() {
        bail!("posts path must exist at: {}", templates_path.display());
    }
    info!(
        "loading templates in {}",
        templates_path.canonicalize()?.display(),
    );
    let mut hb = Handlebars::new();
    hb.set_strict_mode(true);
    load_templates(&templates_path, &mut hb)?;
    debug!("hb: {:?}", hb);

    let posts_path = cli.input.join("posts");
    if !posts_path.exists() {
        bail!("posts path must exist at: {}", posts_path.display());
    }
    info!("loading posts in {}", posts_path.canonicalize()?.display(),);
    let mut posts = Vec::new();
    load_posts(&posts_path, &mut posts)?;

    debug!("posts: {:?}", posts);

    let posts_output_path = cli.output.join("posts");
    fs::create_dir_all(&posts_output_path)?;

    for post in posts {
        let mut filename = posts_output_path.join(post.front_matter.title.to_case(Case::Snake));
        filename.set_extension("html");
        info!("writing {}", filename.display());
        let f = fs::File::create(&filename)?;
        hb.render_to_write("post", &post, f)?;
    }

    Ok(())
}

fn load_templates(dir: &Path, reg: &mut Handlebars) -> Result<()> {
    if dir.is_dir() {
        for entry in dir.read_dir()? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                load_templates(&path, reg)?;
            } else {
                let name = path.file_stem().unwrap().to_str().unwrap();
                let name = name.split_once('.').map(|(name, _)| name).unwrap();
                reg.register_template_file(name, &path)?;
            }
        }
    }

    Ok(())
}

fn load_posts(dir: &Path, posts: &mut Vec<Post>) -> Result<()> {
    if dir.is_dir() {
        for entry in dir.read_dir()? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                load_posts(&path, posts)?;
            } else {
                let contents = fs::read_to_string(&path)?;
                let mut parts = contents.split("---\n").fold(Vec::new(), |mut vec, part| {
                    if !part.is_empty() {
                        vec.push(String::from(part.trim()));
                    }
                    vec
                });

                assert!(parts.len() == 2);
                parts.reverse();

                let front_matter: FrontMatter = serde_yaml::from_str(&parts.pop().unwrap())?;

                let raw_body = parts.pop().unwrap();
                let md_parser = Markdown::new_ext(&raw_body, Options::all());
                let mut body = String::new();
                html::push_html(&mut body, md_parser);

                posts.push(Post { front_matter, body });
            }
        }
    }

    Ok(())
}

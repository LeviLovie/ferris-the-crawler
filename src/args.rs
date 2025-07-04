use clap::{Parser, Subcommand, ValueHint};

#[derive(Parser, Clone, Debug)]
#[command(version, about)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,

    #[clap(
        short = 'u',
        long = "url",
        help = "The URL to start crawling from",
        value_hint = ValueHint::Url,
    )]
    pub url: String,

    #[clap(
        short = 'd',
        long = "depth",
        help = "The maximum depth to crawl",
        default_value = "1"
    )]
    pub depth: u32,

    #[clap(
        short = 'o',
        long = "output",
        help = "The output file to save the crawled data",
        value_hint = ValueHint::FilePath
    )]
    pub output: Option<String>,

    #[clap(
        short = 'q',
        long = "ignore-query",
        help = "Store the query parameters in the urls",
        default_value = "false"
    )]
    pub ignore_query: bool,

    #[clap(
        short = 'f',
        long = "f",
        help = "Add a part of URL to filter the crawled URLs"
    )]
    pub filters: Vec<String>,

    #[clap(
        short = 'i',
        long = "ignore",
        help = "Ignore URLs that match the given patterns"
    )]
    pub ignore: Vec<String>,

    #[clap(
        short = 't',
        long = "threads",
        help = "Number of threads to use for crawling",
        default_value = "1"
    )]
    pub threads: u32,

    #[clap(
        short = 'g',
        long = "gephi",
        help = "Gephi server URL for visualization",
        default_value = "http://localhost:8088/workspace1"
    )]
    pub gephi_url: String,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    Html,
    Wiki {
        #[clap(
            short = 'a',
            long = "amount",
            help = "The amount of links to crawl",
            default_value = "10"
        )]
        amount: u32,

        #[clap(
            short = 'n',
            long = "link",
            help = "The link to use by id",
            default_value = "1"
        )]
        link: Option<u32>,
    },
}

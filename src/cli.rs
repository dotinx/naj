use clap::{Parser, Args};

#[derive(Parser, Debug)]
#[command(name = "naj")]
#[command(about = "Git Operations Shell", long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub manage: Option<ManageFlags>,

    // Operation mode args
    // We make profile_id optional so manage flags can exist without it, 
    // but check constraints in main or via clap attributes if possible.
    // required_unless_present("manage_group") makes it required if no manage flag is set.
    #[arg(required_unless_present("manage_group"))]
    pub profile_id: Option<String>,

    #[arg(allow_hyphen_values = true)]
    pub git_args: Vec<String>,
    
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
#[group(id = "manage_group", multiple = false)]
pub struct ManageFlags {
    #[arg(short, long, num_args = 3, value_names = ["NAME", "EMAIL", "ID"])]
    pub create: Option<Vec<String>>, 

    #[arg(short, long, value_name = "ID")]
    pub remove: Option<String>,

    #[arg(short, long, value_name = "ID")]
    pub edit: Option<String>,

    #[arg(short, long)]
    pub list: bool,
}

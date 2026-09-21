//! # blitz-inspect
//!
//! CLI diagnostic and inspection utility for blitz-host.
//! Automatically discovers the active local Blitz host and dumps its semantic tree.

use std::path::PathBuf;

use blitz_host_protocol::InspectRequest;
use blitz_host_transport::DebugClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let explicit_path = args.get(1).map(PathBuf::from);

    println!("=================================================================");
    println!("[blitz-inspect] Blitz Host Live Inspector");
    println!("=================================================================");

    let mut client = match DebugClient::connect_discovered(explicit_path.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error connecting to Blitz host: {e}");
            eprintln!("Make sure a Blitz host is running with `--debug-control` active.");
            std::process::exit(1);
        }
    };

    let desc = client.descriptor().clone();
    println!("Connected to host:");
    println!("  • Renderer        : {} v{}", desc.renderer, desc.renderer_version);
    println!("  • PID             : {}", desc.pid);
    println!("  • Instance ID     : {}", desc.instance_id);
    println!("  • Socket Path     : {}", desc.socket_path);
    println!("  • Protocol Version: {}", desc.protocol_version);
    println!("-----------------------------------------------------------------");

    println!("Requesting semantic DOM snapshot via inspect()...");
    let response = client.inspect(InspectRequest::default())?;

    println!("Received typed InspectResponse:");
    println!("  • Document ID: {}", response.document_id);
    println!("  • Root ID    : {}", response.root_id);
    println!("  • Node Count : {}", response.node_count);
    println!("-----------------------------------------------------------------");
    println!("Hierarchy:");

    for node in &response.nodes {
        let indent = "  ".repeat(if node.id == response.root_id { 0 } else { 1 });
        let id_str = format!("#{}", node.id);
        let tag_str = format!("<{}>", node.tag);
        let dom_id_str = node.dom_id.as_deref().map(|id| format!(" id=\"{}\"", id)).unwrap_or_default();
        let role_str = node.role.as_deref().map(|r| format!(" [role=\"{}\"]", r)).unwrap_or_default();
        let text_str = node.text.as_deref().map(|t| format!(" \"{}\"", t.trim())).unwrap_or_default();
        let bounds_str = node.bounds.map(|b| format!(" rect({:.1}, {:.1}, {:.1}, {:.1})", b[0], b[1], b[2], b[3])).unwrap_or_default();

        println!(
            "{}{} {}{}{}{}{}",
            indent, id_str, tag_str, dom_id_str, role_str, text_str, bounds_str
        );
    }

    println!("=================================================================");
    println!("[blitz-inspect] Inspection completed successfully!");

    Ok(())
}

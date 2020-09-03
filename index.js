// For more comments about what's going on here, check out the `hello_world`
// example.

async function main(){
    window.pkg = await import('./pkg');
    window.arc_shader = pkg.get_arc_shader();
    // window.line_shader = pkg.get_line_shader();
    window.cubic_shader = pkg.get_cubic_shader();
    window.context = pkg.get_context();
    window.font = await pkg.read_font();
}

main().catch(console.error);
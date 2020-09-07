import { App } from "./app.js";

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}


async function main(){
    window.pkg = await import("./pkg");
    window.Vec2 = pkg.Vec2;
    window.Vec4 = pkg.Vec4;

    window.canvasElement = document.querySelector("canvas");
    window.App = App;
    window.app = new App(pkg, canvasElement);

    // window.arc_shader = pkg.get_arc_shader();
    window.line_shader = pkg.get_line_shader();
    window.cubic_shader = pkg.get_cubic_shader();
    window.context = pkg.get_context();
    window.font = await pkg.read_font();

    
}

main().catch(console.error);
import init, {md_to_md} from './obm.js';


let ready = init()

onmessage = async (e) => {
    await ready
    const {version, source} = e.data;
    let output = md_to_md(source, 80, 80)
    postMessage({version, output})
}
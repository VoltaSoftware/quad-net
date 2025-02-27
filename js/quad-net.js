function on_init() {
}

register_plugin = function (importObject) {
    importObject.env.ws_connect = ws_connect;
    importObject.env.ws_send = ws_send;
    importObject.env.ws_close = ws_close;
    importObject.env.ws_try_recv = ws_try_recv;

    importObject.env.http_make_request = http_make_request;
    importObject.env.http_try_recv = http_try_recv;
}

miniquad_add_plugin({register_plugin, on_init, version: 1, name: "quad_net"});

var quad_socket;
var received_buffer = [];

const Connected = 0;
const PackedReceived = 1;
const SocketError = 2;
const Closed = 3;

function ws_connect(a) {
    received_buffer = [];

    let addr = consume_js_object(a);
    console.error("Connection to", addr);

    quad_socket = new WebSocket(addr);
    quad_socket.binaryType = 'arraybuffer';
    quad_socket.onopen = function () {
        received_buffer.push({
            "type": Connected,
        });
    };

    quad_socket.onmessage = function (msg) {
        if (typeof msg.data == "string") {
            console.error("Received string data: ", msg.data);
        } else {
            const buffer = new Uint8Array(msg.data);
            received_buffer.push({
                "type": PackedReceived,
                "data": buffer
            });
        }
    };

    quad_socket.onerror = function (error) {
        console.error("Websocket error:", error);
        received_buffer.push({
            "type": SocketError,
            "data": JSON.stringify(error)
        });
    };

    quad_socket.onclose = function () {
        received_buffer.push({
            "type": Closed,
        });
    };
}

function ws_close() {
    console.error("Closing websocket connection by request");
    quad_socket.close();
}

function ws_send(data) {
    try {
        const array = consume_js_object(data);
        // here should be a nice typecheck on array.is_string or whatever
        if (array.buffer !== undefined) {
            quad_socket.send(array.buffer);
        } else {
            quad_socket.send(array);
        }
    } catch (error) {
        console.error("Error sending data: ", error);  // Convert error to string and log

        const error_message = error.message;

        received_buffer.push({
            "type": SocketError,
            "data": JSON.stringify(error_message)
        });
    }
}

function ws_try_recv() {
    if (received_buffer.length !== 0) {
        return js_object(received_buffer.shift())
    }
    return -1;
}


let uid = 0;
const ongoing_requests = {};

function http_try_recv(cid) {
    if (ongoing_requests[cid] !== undefined && ongoing_requests[cid] != null) {
        var data = ongoing_requests[cid];
        ongoing_requests[cid] = null;
        return js_object(data);
    }
    return -1;
}

function http_make_request(scheme, url, body, headers) {
    const cid = uid;

    uid += 1;

    let scheme_string;
    if (scheme === 0) {
        scheme_string = 'POST';
    }
    if (scheme === 1) {
        scheme_string = 'PUT';
    }
    if (scheme === 2) {
        scheme_string = 'GET';
    }
    if (scheme === 3) {
        scheme_string = 'DELETE';
    }
    var url_string = consume_js_object(url);
    var body_string = consume_js_object(body);
    var headers_obj = consume_js_object(headers);
    var xhr = new XMLHttpRequest();
    xhr.open(scheme_string, url_string, true);
    xhr.responseType = 'arraybuffer';
    for (const header in headers_obj) {
        xhr.setRequestHeader(header, headers_obj[header]);
    }
    xhr.onload = function (e) {
        if (this.status === 200) {
            ongoing_requests[cid] = new Uint8Array(this.response);
        }
    }
    xhr.onerror = function (e) {
        // todo: let rust know and put Error to ongoing requests
        console.error("Failed to make a request");
        console.error(e);
    };

    xhr.send(body_string);

    return cid;
}

const http = require('http')

const n = 1000;
let connected = 0;
let messages = 0;
let start = Date.now();
let phase = 'connecting';
let connection_time;
let broadcast_time;

let message = process.argv[2] || 'msg';
let expected_data = "data: " + message;

for (let i = 0; i < n; i++) {
    http.get({
        host: 'localhost',
        port: 8080,
        path: '/events/channel1'
    }, response => {
        response.on('data', data => {
            if (data.includes(expected_data)) {
                messages += 1;
            } else if (data.includes("data: connected\n")) {
                connected += 1;
            }
        })
    }).on('error', (_) => {});
}

setInterval(() => {
    if (phase === 'connecting' && connected === n) {
        // done connecting
        phase = 'messaging';
        connection_time = Date.now() - start;
    }

    if (phase === 'messaging') {
        phase = 'waiting';
        start = Date.now();

        postData = JSON.stringify({
            channel: "channel1",
            message: message
        })

        options = {
            host: 'localhost',
            port: 8080,
            path: '/broadcast/',
            method: "POST",
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
                'Content-Length': Buffer.byteLength(postData),
            },
        };

        var req = http.request(options, res => {
            res.on('data', _ => {})
        })

        req.on("error", e => console.error(`problem with request ${e.msg}`));
        req.write(postData);
        req.end();
    }

    if (phase === 'waiting' && messages >= n) {
        // all messages received
        broadcast_time = Date.now() - start;
        phase = 'paused';
        messages = 0;
        phase = 'messaging';
    }

    process.stdout.write("\r\x1b[K");
    process.stdout.write(`Connected: ${connected}, connection time: ${connection_time} ms, total broadcast time: ${broadcast_time} ms`);
}, 20)

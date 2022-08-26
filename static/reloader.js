const reload_html = `
<div class="container py-5 px-5">
    <div class="column is-10 is-offset-1">
        Waiting for server restart...
        <progress id="progbar" class="progress is-info" max="100"></progress>
    </div>
</div>
`

const quit_html = `
<div class="container py-5 px-5">
    <div class="column is-10 is-offset-1">
        <span id="text">Shutting down server...</span>
        <progress class="progress is-info" max="100"></progress>
    </div>
</div>
`

function quitter() {
    document.body.innerHTML = quit_html;

    setTimeout(() => {
        let loop = setInterval(function () {
            try_not_get(window.location.href, () => {
                clearInterval(loop);
                document.getElementById("progbar").value = 100;
                document.getElementById("text").innerText = "Server shutdown complete";
            });
        }, 2000);

    }, 3000);
}

function try_not_get(url, then) {
    const XHR = new XMLHttpRequest();
    XHR.open("GET", url);
    XHR.send();
    XHR.onerror = function () {
        then();
    }
}


/// Pings server until url becomes available, then redirects
function reloader(url) {
    document.body.innerHTML = reload_html;

    setTimeout(() => {
        let loop = setInterval(function () {
            try_get(url, () => {
                clearInterval(loop);
                window.location.assign(url);
            });
        }, 2000);

    }, 3000);
}

function try_get(url, then) {
    const XHR = new XMLHttpRequest();
    XHR.open("GET", url);
    XHR.send();
    XHR.onload = function () {
        if (this.status === 200) {
            then();
        };
    };
}
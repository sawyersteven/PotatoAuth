let alert_container;
let alert_success_template;
let alert_warning_template;
let alert_error_template;


document.addEventListener("DOMContentLoaded", function (event) {
    alert_container = document.querySelector("#alert_container");
    alert_container.style.position = "absolute";
    alert_container.style.bottom = "1em";
    alert_container.style.zIndex = 9999;

    let d = document.createElement("div");

    d.innerHTML = alert_warning_html;
    alert_warning_template = d.children[0];

    d.innerHTML = alert_success_html;
    alert_success_template = d.children[0];

    d.innerHTML = alert_error_html;
    alert_error_template = d.children[0];
});

alert_error_html = `
<div class="notification is-danger">
    <button class="delete"></button>
    <span class="alert_content"></span>
</div>
`

alert_warning_html = `
<div class="notification is-warning">
    <button class="delete"></button>
    <span class="alert_content"></span>
</div>
`

alert_success_html = `
<div class="notification is-success">
    <button class="delete"></button>
    <span class="alert_content"></span>
</div>
`

function push_alert(content, kind) {
    let tmpl;
    if (kind == "warning") { tmpl = alert_warning_template.cloneNode(true); }
    else if (kind == "error") { tmpl = alert_error_template.cloneNode(true); }
    else if (kind == "success") { tmpl = alert_success_template.cloneNode(true); }
    if (tmpl === null) console.error(`${kind} is not a valid alert`);

    content = content.charAt(0).toUpperCase() + content.slice(1);

    tmpl.querySelector("span.alert_content").innerText = content;
    tmpl.querySelector("button.delete").addEventListener("click", function (event) {
        this.parentElement.remove();
    });
    alert_container.appendChild(tmpl);
}
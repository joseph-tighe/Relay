function createHash(string) {
    let hash = 0;

    if (string.length === 0) {
        return hash;
    }

    for (let i = 0; i < string.length; i++) {
        const char = string.charCodeAt(i);

        hash = ((hash << 5) - hash) + char;
        hash |= 0;
    }

    return hash;
}
async function readJsonResponse(response) {
    const body = await response.text();
    if (!response.ok) {
        throw new Error(`Request failed (${response.status}): ${body}`);
    }
    return JSON.parse(body);
}
function formValue(form, fieldName) {
    const el = form.querySelector(`[name="${fieldName}"]`);
    return el ? el.value : "";
}
function showFormError(form, message) {
    let el = form.querySelector(".form-error");
    if (!el) {
        el = document.createElement("p");
        el.className = "form-error";
        form.appendChild(el);
    }
    el.textContent = message;
}
function cleanErrorMessage(error) {
    return String(error.message || error).replace(/^Request failed \(\d+\): /, "");
}
function clearFormErrors() {
    document.querySelectorAll(".form-error").forEach(el => el.remove());
}
document.getElementsByClassName("form-signup")[0].getElementsByTagName("button")[0].addEventListener("click", e => {
    const form = document.getElementsByClassName("form-signup")[0];
    // NOTE: form.name is the form's own name prop, NOT the Name input.
    // Must use querySelector or form.elements.
    let name = formValue(form, "name").trim();
    let email = formValue(form, "email").trim();
    let password = formValue(form, "password");
    if (!name || !email || !password) {
        showFormError(form, "Please fill in name, email, and password.");
        return;
    }
    let passwordHash = String(createHash(password));
    console.log(JSON.stringify({
            "name": name,
            "email": email,
            "password": passwordHash
        }))
    fetch("/api/signup", {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            "name": name,
            "email": email,
            "password": passwordHash
        })
    }).then(readJsonResponse)
        .then(user => {
            sessionStorage.setItem("userId", user.id);
            sessionStorage.setItem("passHash", passwordHash);
            window.location.href = "/";
        }).catch(error => {
            console.error("Signup failed:", error);
            showFormError(form, cleanErrorMessage(error));
        });
});
document.getElementsByClassName("form-login")[0].getElementsByTagName("button")[0].addEventListener("click", e => {
    const form = document.getElementsByClassName("form-login")[0];
    let email = formValue(form, "email").trim();
    let password = formValue(form, "password");
    if (!email || !password) {
        showFormError(form, "Please fill in email and password.");
        return;
    }
    let passwordHash = String(createHash(password));
    fetch("/api/login", {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            "email": email,
            "password": passwordHash
        })
    }).then(readJsonResponse)
        .then(user => {
            sessionStorage.setItem("userId", user.id);
            sessionStorage.setItem("passHash", passwordHash);
            window.location.href = "/";
        }).catch(error => {
            console.error("Login failed:", error);
            showFormError(form, cleanErrorMessage(error));
        });
});
document.getElementById("tab-login").addEventListener("change", clearFormErrors);
document.getElementById("tab-signup").addEventListener("change", clearFormErrors);
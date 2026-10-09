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
document.getElementsByClassName("form-signup")[0].getElementsByTagName("button")[0].addEventListener("click", e => {
    let name = document.getElementsByClassName("form-signup")[0].name.value;
    let email = document.getElementsByClassName("form-signup")[0].email.value;
    let password = document.getElementsByClassName("form-signup")[0].password.value;
    if (!name || !email || !password) {
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
        }).catch(error => console.error("Signup failed:", error));
});
document.getElementsByClassName("form-login")[0].getElementsByTagName("button")[0].addEventListener("click", e => {
    let email = document.getElementsByClassName("form-login")[0].email.value;
    let password = document.getElementsByClassName("form-login")[0].password.value;
    if (!email || !password) {
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
        }).catch(error => console.error("Login failed:", error));
});
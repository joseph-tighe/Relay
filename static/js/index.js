let userId = parseInt(sessionStorage.getItem("userId"));
if (!userId) {
    window.location.href = "/login";
}
function getUserById(id) {
    return fetch(`/api/user/${id}`)
        .then(res => res.json())
        .then(user => user);
}
function getClubsByUserId(userId) {
    return fetch(`/api/clubs-in/${userId}`)
        .then(res => res.json())
        .then(clubs => clubs);
}
function getUser(id) {
    if (IdToUser[id]) {
        return IdToUser[id];
    } else {
        IdToUser[id] = getUserById(id);
        return IdToUser[id];
    }
}
function getClubById(id) {
    return fetch(`/api/club/${id}`)
        .then(res => res.json())
        .then(club => club);
}
function changeMyName(name) {
    document.querySelector(".username").innerText = name;
    document.querySelector(".me").innerText = name[0];
}
function getMessagesByClubId(clubId) {
    return fetch(`/api/club/${clubId}/messages`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            "passwordhash": sessionStorage.getItem("passHash"),
            "user_id": userId
        })
    })
        .then(res => res.json())
        .then(messages => messages);
}
function addClub(name, image) {
    let club = document.createElement("div");
    club.classList.add("club");
    let clubName = document.createElement("div");
    clubName.classList.add("club-name");
    clubName.innerText = name;
    club.appendChild(clubName);/*
    let clubImage = document.createElement("img");
    clubImage.classList.add("club-image");
    clubImage.src = image;
    club.appendChild(clubImage);*/
    document.querySelector(".rooms").appendChild(club);
}
let userData = await getUserById(userId);
changeMyName(userData.name);
let clubsIds = await getClubsByUserId(userId).then(clubsIds => clubsIds.clubs);
let clubs = [];
for (let clubId of clubsIds) {
    clubs.push(await getClubById(clubId));
}
let messages = [];
for (let club of clubs) {
    console.log(club.name);
    addClub(club.name, club.image);
    messages.push(await getMessagesByClubId(clubsIds[clubs.indexOf(club)]).then(messages => messages.messages));
}
async function renderMessage(message) {
    let user = await getUser(message.userId);
    let messageDiv = document.createElement("div");
    messageDiv.classList.add("msg");
    let ava = document.createElement("div");
    ava.classList.add("ava", "a" + ((message.userId % 5) + 1));
    ava.innerText = user.name[0];
    messageDiv.appendChild(ava);
    let body = document.createElement("div");
    messageDiv.appendChild(body);
    let meta = document.createElement("div");
    meta.classList.add("meta");
    let strong = document.createElement("strong");
    strong.innerText = user.name;
    meta.appendChild(strong);
    let time = document.createElement("time");
    time.title = message.timestamp;
    let short = message.timestamp.slice(11, 16);
    time.innerText = /^\d{2}:\d{2}$/.test(short) ? short : message.timestamp;
    meta.appendChild(time);
    body.appendChild(meta);
    let p = document.createElement("p");
    p.innerText = message.text;
    body.appendChild(p);
    document.querySelector(".scroll").appendChild(messageDiv);
}
let IdToUser = {userId: userData};
if (messages.length > 0) {
    for (let message of messages[0]) {
        console.log(message.text, message.userId, message.timestamp);
        await renderMessage(message);
    }
}
document.getElementById("message-input").addEventListener("keydown", e => {
    if (e.key === "Enter") {
        e.preventDefault();
        let messageInput = document.getElementById("message-input");
        let message = messageInput.value;
        console.log(JSON.stringify({
                "text": message,
                "user_id": userId
            }));
        fetch(`/api/club/${clubsIds[0]}/messages/new`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                "password": sessionStorage.getItem("passHash"),
                "text": message,
                "user_id": userId
            })
        }).then(res => res.json())
            .then(async message => {
                await renderMessage(message);
                messageInput.value = "";
            });
        messageInput.focus();
    }
});
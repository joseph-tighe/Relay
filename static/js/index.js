let userId = parseInt(sessionStorage.getItem("userId"));
if (!userId || !sessionStorage.getItem("passHash")) {
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
const IdToUser = {};
function getClubById(id) {
    return fetch(`/api/club/${id}`)
        .then(res => res.json())
        .then(club => club);
}
function changeMyName(name) {
    document.querySelector(".username").innerText = name;
    document.querySelector(".me").innerText = name[0];
}
function authBody() {
    return {
        "passwordhash": sessionStorage.getItem("passHash"),
        "user_id": userId
    };
}
function getSubspacesByClubId(clubId) {
    return fetch(`/api/club/${clubId}/subspaces`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify(authBody())
    })
        .then(res => res.json())
        .then(data => data);
}
function getMessagesBySubspaceId(subspaceId) {
    return fetch(`/api/subspace/${subspaceId}/messages`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify(authBody())
    })
        .then(res => res.json())
        .then(data => data);
}
let activeSubspaceId = null;
const seenMessageIds = new Set();
const subspaceLeaves = {};
function subspaceTree(subspaces) {
    const byId = {};
    const roots = [];
    for (let subspace of subspaces) {
        byId[subspace.id] = { ...subspace, children: [] };
    }
    for (let id in byId) {
        const node = byId[id];
        if (node.parent_id && byId[node.parent_id]) {
            byId[node.parent_id].children.push(node);
        } else {
            roots.push(node);
        }
    }
    return roots;
}
function renderSubspace(bodyEl, subspace, depth) {
    let leaf = document.createElement("div");
    leaf.classList.add("leaf");
    if (depth > 0) {
        leaf.classList.add("deep-leaf");
    }
    leaf.innerText = subspace.name;
    leaf.addEventListener("click", () => selectSubspace(subspace.id));
    bodyEl.appendChild(leaf);
    subspaceLeaves[subspace.id] = leaf;
    for (let child of subspace.children) {
        renderSubspace(bodyEl, child, depth + 1);
    }
}
function addClub(club, subspaces) {
    if (typeof club === "string") {
        club = { name: club };
    }
    subspaces = subspaces || [];
    const rooms = document.querySelector(".rooms");
    let clubEl = document.createElement("div");
    clubEl.classList.add("club");
    let clubName = document.createElement("div");
    clubName.classList.add("club-name");
    clubName.innerText = club.name;
    clubEl.appendChild(clubName);
    let bodyEl = document.createElement("div");
    bodyEl.classList.add("club-body");
    for (let root of subspaceTree(subspaces)) {
        renderSubspace(bodyEl, root, 0);
    }
    clubEl.addEventListener("click", (e) => {
        if (e.target.closest(".leaf")) return;
        bodyEl.classList.toggle("collapsed");
    });
    clubEl.appendChild(bodyEl);
    rooms.appendChild(clubEl);
}
function clearFeed() {
    document.querySelector(".scroll").innerHTML = "";
    seenMessageIds.clear();
}
async function loadMessages() {
    if (!activeSubspaceId) return;
    const data = await getMessagesBySubspaceId(activeSubspaceId);
    for (let message of data.messages) {
        if (seenMessageIds.has(message.id)) continue;
        seenMessageIds.add(message.id);
        await renderMessage(message);
    }
}
async function selectSubspace(subspaceId) {
    activeSubspaceId = subspaceId;
    document.querySelectorAll(".leaf.current").forEach(el => el.classList.remove("current"));
    if (subspaceLeaves[subspaceId]) {
        subspaceLeaves[subspaceId].classList.add("current");
    }
    clearFeed();
    await loadMessages();
}
let userData = await getUserById(userId);
changeMyName(userData.name);
let clubsIds = await getClubsByUserId(userId).then(clubsIds => clubsIds.clubs);
let firstSubspaceId = null;
for (let clubId of clubsIds) {
    const club = await getClubById(clubId);
    const data = await getSubspacesByClubId(clubId);
    addClub(club, data.subspaces);
    if (firstSubspaceId === null && data.subspaces.length > 0) {
        firstSubspaceId = data.subspaces[0].id;
    }
}
if (firstSubspaceId !== null) {
    await selectSubspace(firstSubspaceId);
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
document.getElementById("message-input").addEventListener("keydown", e => {
    if (e.key === "Enter") {
        e.preventDefault();
        if (!activeSubspaceId) return;
        let messageInput = document.getElementById("message-input");
        let message = messageInput.value;
        if (!message.trim()) {
            messageInput.focus();
            return;
        }
        fetch(`/api/subspace/${activeSubspaceId}/messages/new`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                "passwordhash": sessionStorage.getItem("passHash"),
                "text": message,
                "user_id": userId
            })
        }).then(res => res.json())
            .then(async message => {
                if (!seenMessageIds.has(message.id)) {
                    seenMessageIds.add(message.id);
                    await renderMessage(message);
                }
                messageInput.value = "";
            });
        messageInput.focus();
    }
});
setInterval(async () => {
    await loadMessages();
}, 1000);
function createClub() {
    let name = prompt("Club name");
    let image = prompt("Club image URL");
    if (!name) {
        return;
    }
    if (!image) {
        image = "null";
    }
    fetch(`/api/club/new`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            "name": name,
            "image": image,
            "owner_id": userId,
            "owner_password": sessionStorage.getItem("passHash")
        })
    }).then(res => res.json())
        .then(async data => {
            const club = await getClubById(data.club_id);
            const subspaces = await getSubspacesByClubId(data.club_id);
            addClub(club, subspaces.subspaces);
        });
}
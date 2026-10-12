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
        const willOpen = bodyEl.classList.contains("collapsed");
        document.querySelectorAll(".club").forEach(el => {
            el.classList.remove("open");
            el.querySelector(".club-body").classList.add("collapsed");
        });
        if (willOpen) {
            clubEl.classList.add("open");
            bodyEl.classList.remove("collapsed");
        }
    });
    clubEl.appendChild(bodyEl);
    rooms.appendChild(clubEl);
    return clubEl;
}
function clearFeed() {
    document.querySelector(".scroll").innerHTML = "";
    seenMessageIds.clear();
}
let loadMessagesInFlight = false;
async function loadMessages() {
    if (!activeSubspaceId || loadMessagesInFlight) return;
    if (document.hidden) return;
    loadMessagesInFlight = true;
    try {
        const data = await getMessagesBySubspaceId(activeSubspaceId);
        for (let message of data.messages) {
            if (seenMessageIds.has(message.id)) continue;
            seenMessageIds.add(message.id);
            await renderMessage(message);
        }
    } finally {
        loadMessagesInFlight = false;
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
let firstClubEl = null;
for (let clubId of clubsIds) {
    const club = await getClubById(clubId);
    const data = await getSubspacesByClubId(clubId);
    const clubEl = addClub(club, data.subspaces);
    if (firstClubEl === null) {
        firstClubEl = clubEl;
    }
    if (firstSubspaceId === null && data.subspaces.length > 0) {
        firstSubspaceId = data.subspaces[0].id;
    }
}
document.querySelectorAll(".club").forEach(el => {
    if (el === firstClubEl) {
        el.classList.add("open");
    } else {
        el.querySelector(".club-body").classList.add("collapsed");
    }
});
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
}, 3000);
async function createClub(name, image) {
    if (name === undefined) {
        name = prompt("Club name");
        if (!name) {
            return;
        }
        image = prompt("Club image URL");
    }
    if (!image) {
        image = "null";
    }
    const res = await fetch(`/api/club/new`, {
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
    });
    if (!res.ok) {
        throw new Error(await res.text());
    }
    const data = await res.json();
    const club = await getClubById(data.club_id);
    const subspaces = await getSubspacesByClubId(data.club_id);
    addClub(club, subspaces.subspaces);
    return data;
}
async function createSubspace(clubId, name, parentId = null) {
    const res = await fetch(`/api/subspace/new`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            "name": name,
            "parent_id": parentId,
            "club_id": clubId,
            "owner_id": userId,
            "owner_password": sessionStorage.getItem("passHash")
        })
    });
    if (!res.ok) {
        throw new Error(await res.text());
    }
    return res.json();
}
async function addClubMember(clubId, email) {
    const res = await fetch(`/api/club/${clubId}/members`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            "email": email,
            "user_id": userId,
            "passwordhash": sessionStorage.getItem("passHash")
        })
    });
    if (!res.ok) {
        throw new Error(await res.text());
    }
    return res.json();
}
// Expected contract for the upcoming endpoint:
// POST /api/clubs/mine with {user_id, passwordhash} -> {"clubs": [{id, name, ...}]}
// (bare [ids] also tolerated and resolved via /api/club/{id})
async function getOwnedClubs() {
    const res = await fetch(`/api/clubs/mine`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify(authBody())
    });
    if (!res.ok) {
        throw new Error("clubs/mine failed: " + res.status);
    }
    return res.json();
}
function renderOwnedSubspace(body, subspace, depth) {
    const sub = document.createElement("div");
    sub.classList.add("owned-sub");
    if (depth > 0) {
        sub.classList.add("deep");
    }
    sub.innerText = subspace.name;
    body.appendChild(sub);
    for (let child of subspace.children) {
        renderOwnedSubspace(body, child, depth + 1);
    }
}
function flattenSubspaceOptions(subspaces) {
    const out = [];
    function walk(nodes, depth) {
        for (let node of nodes) {
            out.push({ id: node.id, label: "— ".repeat(depth) + node.name });
            walk(node.children || [], depth + 1);
        }
    }
    walk(subspaceTree(subspaces), 0);
    return out;
}
async function loadOwnedClubs() {
    const box = document.getElementById("owned-clubs");
    box.innerHTML = "";
    let items;
    try {
        items = (await getOwnedClubs()).clubs || [];
    } catch {
        box.innerHTML = '<p class="muted">Could not load owned clubs.</p>';
        return;
    }
    if (!items.length) {
        box.innerHTML = '<p class="muted">No owned clubs yet.</p>';
        return;
    }
    let firstOwned = true;
    for (let item of items) {
        const clubId = item.id ?? item;
        const club = item.name ? item : await getClubById(clubId);
        let subspaces = [];
        try {
            subspaces = (await getSubspacesByClubId(clubId)).subspaces || [];
        } catch {
            subspaces = [];
        }
        const row = document.createElement("div");
        row.classList.add("owned-club");
        const head = document.createElement("div");
        head.classList.add("owned-head");
        const title = document.createElement("strong");
        title.innerText = club.name;
        head.appendChild(title);
        row.appendChild(head);
        const body = document.createElement("div");
        body.classList.add("owned-body");
        for (let root of subspaceTree(subspaces)) {
            renderOwnedSubspace(body, root, 0);
        }
        const form = document.createElement("div");
        form.classList.add("new-sub");
        const input = document.createElement("input");
        input.type = "text";
        input.placeholder = "New subspace";
        const parentSelect = document.createElement("select");
        parentSelect.setAttribute("aria-label", "Parent subspace");
        const topOption = document.createElement("option");
        topOption.value = "";
        topOption.innerText = "Top level";
        parentSelect.appendChild(topOption);
        for (let option of flattenSubspaceOptions(subspaces)) {
            const el = document.createElement("option");
            el.value = String(option.id);
            el.innerText = option.label;
            parentSelect.appendChild(el);
        }
        const add = document.createElement("button");
        add.type = "button";
        add.innerText = "Add";
        add.addEventListener("click", async () => {
            const subName = input.value.trim();
            if (!subName) return;
            const parentId = parentSelect.value ? parseInt(parentSelect.value) : null;
            try {
                await createSubspace(clubId, subName, parentId);
                window.location.reload();
            } catch (err) {
                showModalError(err.message);
            }
        });
        form.appendChild(input);
        form.appendChild(parentSelect);
        form.appendChild(add);
        body.appendChild(form);
        const memberForm = document.createElement("div");
        memberForm.classList.add("new-sub");
        const emailInput = document.createElement("input");
        emailInput.type = "email";
        emailInput.placeholder = "Add user by email";
        const invite = document.createElement("button");
        invite.type = "button";
        invite.innerText = "Add";
        invite.addEventListener("click", async () => {
            const email = emailInput.value.trim();
            if (!email) return;
            try {
                await addClubMember(clubId, email);
                window.location.reload();
            } catch (err) {
                showModalError(err.message);
            }
        });
        memberForm.appendChild(emailInput);
        memberForm.appendChild(invite);
        body.appendChild(memberForm);
        head.addEventListener("click", () => {
            const willOpen = body.classList.contains("collapsed");
            box.querySelectorAll(".owned-club").forEach(el => {
                el.classList.remove("open");
                el.querySelector(".owned-body").classList.add("collapsed");
            });
            if (willOpen) {
                row.classList.add("open");
                body.classList.remove("collapsed");
            }
        });
        row.appendChild(body);
        if (firstOwned) {
            firstOwned = false;
            row.classList.add("open");
        } else {
            body.classList.add("collapsed");
        }
        box.appendChild(row);
    }
}
const settingsModal = document.getElementById("settings-modal");
function openSettings() {
    settingsModal.hidden = false;
    loadOwnedClubs();
}
function closeSettings() {
    settingsModal.hidden = true;
}
function showModalError(message) {
    let el = document.getElementById("modal-error");
    if (!el) {
        el = document.createElement("p");
        el.id = "modal-error";
        el.className = "form-error";
        document.querySelector(".modal").appendChild(el);
    }
    el.textContent = message;
    el.hidden = false;
}
document.getElementById("settings-btn").addEventListener("click", openSettings);
document.getElementById("settings-close").addEventListener("click", closeSettings);
settingsModal.addEventListener("click", (e) => {
    if (e.target === settingsModal) {
        closeSettings();
    }
});
document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !settingsModal.hidden) {
        closeSettings();
    }
});
document.getElementById("new-club-btn").addEventListener("click", async () => {
    const nameInput = document.getElementById("new-club-name");
    const imageInput = document.getElementById("new-club-image");
    const name = nameInput.value.trim();
    if (!name) {
        return;
    }
    try {
        await createClub(name, imageInput.value.trim());
        window.location.reload();
    } catch (err) {
        showModalError(err.message);
    }
});
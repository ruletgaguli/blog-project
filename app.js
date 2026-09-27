import init, { BlogApp } from "./blog-wasm/pkg/blog_wasm.js";

await init();
const app = new BlogApp();
const byId = (id) => document.getElementById(id);
const state = { mode: "login", editing: null, posts: [] };

function notice(message = "") { byId("notice").textContent = String(message); }
function errorMessage(error) { return typeof error === "string" ? error : String(error); }

function updateAuth() {
  const authenticated = app.is_authenticated();
  byId("auth-panel").hidden = authenticated;
  byId("auth-status").textContent = authenticated ? "Вы вошли" : "Гость";
  byId("new-post").hidden = !authenticated;
  byId("logout").hidden = !authenticated;
  if (!authenticated) closeEditor();
  renderPosts();
}

function setMode(mode) {
  state.mode = mode;
  const register = mode === "register";
  byId("email-field").hidden = !register;
  byId("email").required = register;
  byId("password").autocomplete = register ? "new-password" : "current-password";
  byId("auth-submit").textContent = register ? "Зарегистрироваться" : "Войти";
  for (const name of ["login", "register"]) {
    const tab = byId(`${name}-tab`);
    tab.classList.toggle("active", name === mode);
    tab.setAttribute("aria-selected", String(name === mode));
  }
}

function closeEditor() {
  state.editing = null;
  byId("editor-panel").hidden = true;
  byId("post-form").reset();
}

function openEditor(post = null) {
  state.editing = post?.id ?? null;
  byId("editor-title").textContent = post ? "Редактировать пост" : "Новый пост";
  byId("post-submit").textContent = post ? "Сохранить" : "Опубликовать";
  byId("post-title").value = post?.title ?? "";
  byId("post-content").value = post?.content ?? "";
  byId("editor-panel").hidden = false;
  byId("post-title").focus();
}

function button(text, className, handler) {
  const element = document.createElement("button");
  element.type = "button";
  element.className = `button ${className}`;
  element.textContent = text;
  element.addEventListener("click", handler);
  return element;
}

function renderPosts() {
  const container = byId("posts");
  container.replaceChildren();
  if (state.posts.length === 0) {
    const empty = document.createElement("p");
    empty.className = "empty";
    empty.textContent = "Публикаций пока нет";
    container.append(empty);
    return;
  }
  for (const post of state.posts) {
    const article = document.createElement("article");
    article.className = "post";
    const head = document.createElement("div");
    head.className = "post-head";
    const text = document.createElement("div");
    const title = document.createElement("h2");
    title.className = "post-title";
    title.textContent = post.title;
    const meta = document.createElement("div");
    meta.className = "post-meta";
    meta.textContent = `${new Date(post.created_at).toLocaleString("ru-RU")} · Автор #${post.author_id}`;
    text.append(title, meta);
    head.append(text);
    if (app.is_authenticated() && app.current_user_id() === BigInt(post.author_id)) {
      const actions = document.createElement("div");
      actions.className = "post-actions";
      actions.append(button("Изменить", "quiet", () => openEditor(post)));
      actions.append(button("Удалить", "danger", async () => {
        if (!confirm(`Удалить «${post.title}»?`)) return;
        try { await app.delete_post(BigInt(post.id)); await loadPosts(); notice("Пост удалён"); }
        catch (error) { notice(errorMessage(error)); }
      }));
      head.append(actions);
    }
    const content = document.createElement("p");
    content.className = "post-content";
    content.textContent = post.content;
    article.append(head, content);
    container.append(article);
  }
}

async function loadPosts() {
  try {
    const result = await app.load_posts();
    state.posts = result.posts ?? [];
    renderPosts();
  } catch (error) { notice(errorMessage(error)); }
}

byId("login-tab").addEventListener("click", () => setMode("login"));
byId("register-tab").addEventListener("click", () => setMode("register"));
byId("reload").addEventListener("click", loadPosts);
byId("new-post").addEventListener("click", () => openEditor());
byId("close-editor").addEventListener("click", closeEditor);
byId("logout").addEventListener("click", () => { app.logout(); updateAuth(); notice(""); });

byId("auth-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  notice("");
  const submit = byId("auth-submit");
  submit.disabled = true;
  try {
    const username = byId("username").value.trim();
    const password = byId("password").value;
    if (state.mode === "register") await app.register(username, byId("email").value.trim(), password);
    else await app.login(username, password);
    byId("auth-form").reset();
    updateAuth();
  } catch (error) { notice(errorMessage(error)); }
  finally { submit.disabled = false; }
});

byId("post-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  notice("");
  const submit = byId("post-submit");
  submit.disabled = true;
  try {
    const title = byId("post-title").value.trim();
    const content = byId("post-content").value.trim();
    if (state.editing !== null) await app.update_post(BigInt(state.editing), title, content);
    else await app.create_post(title, content);
    closeEditor();
    await loadPosts();
  } catch (error) { notice(errorMessage(error)); }
  finally { submit.disabled = false; }
});

updateAuth();
await loadPosts();

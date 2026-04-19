mod git;
use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::Context as _;
use forgejo_api::{
    Auth, Forgejo,
    structs::{CreateLabelOption, CreatePullRequestOption, IssueListLabelsQuery},
};
use indoc::indoc;
use log::info;
use rocket::form::{Form, Strict};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::response::Redirect;
use rocket::response::content::RawHtml;
use rocket::{FromForm, Request, State, get, launch, post, routes};
use serde::{Deserialize, Serialize};
use serde_jsonlines::append_json_lines;
use tempdir::TempDir;

const LOGS_PATH: &str = "/var/log/propener/logs.jsonl";
const REPO_PATH: &str = "/var/lib/propener/repo";

#[derive(FromForm, Serialize)]
struct UserForm {
    email: Option<String>,
    name: Option<String>,
    payload: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Payload {
    patch: String,
    pr_title: String,
    pr_body: String,
}

#[derive(Debug)]
struct PROpenerConfig {
    repo_path: PathBuf,
    forgejo_url: String,
    forgejo_token: String,
    repo_owner: String,
    repo_name: String,
}

#[derive(Serialize)]
struct IpHeader {
    ip: IpAddr,
}

#[derive(Serialize)]
struct PRLog<'a> {
    form: &'a UserForm,
    ip: &'a IpHeader,
    pr_title: &'a str,
    pr_body: &'a str,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for IpHeader {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.client_ip() {
            Some(ip) => Outcome::Success(Self { ip }),
            None => Outcome::Error((Status::InternalServerError, "Request is missing X-Real-IP")),
        }
    }
}

impl PROpenerConfig {
    fn from_env() -> anyhow::Result<Self> {
        let repo_url = std::env::var("PROPENER_REPO_URL").expect("PROPENER_REPO_URL should be set");
        let repo_path = PathBuf::from(
            std::env::var("PROPENER_REPO_PATH").unwrap_or_else(|_| REPO_PATH.to_owned()),
        );
        let forgejo_url = std::env::var("PROPENER_FORGEJO_URL")
            .unwrap_or_else(|_| "https://codeberg.org".to_owned());
        let forgejo_token =
            std::env::var("PROPENER_FORGEJO_TOKEN").expect("PROPENER_FORGEJO_TOKEN should be set");
        let repo_owner =
            std::env::var("PROPENER_REPO_OWNER").expect("PROPENER_REPO_OWNER should be set");
        let repo_name =
            std::env::var("PROPENER_REPO_NAME").expect("PROPENER_REPO_NAME should be set");

        if repo_path.join(".git").exists() {
            info!("Repo exists at {repo_path:?}, fetching...");
            git::fetch_main(&repo_path).context("Failed to fetch")?;
        } else {
            info!("Cloning repo to {repo_path:?}...");
            git::clone_repo(&repo_url, &repo_path).context("Failed to clone repo")?;
        }

        Ok(Self {
            repo_path,
            forgejo_url,
            forgejo_token,
            repo_owner,
            repo_name,
        })
    }
}

#[launch]
fn rocket() -> _ {
    let config = PROpenerConfig::from_env().expect("Config is invalid");
    rocket::build()
        .manage(config)
        .mount("/", routes![index_redirect, submit, submit_page])
}

#[get("/")]
fn index_redirect() -> Redirect {
    Redirect::to("/submit")
}

#[get("/submit")]
fn submit_page(ip: IpHeader) -> RawHtml<String> {
    const PAGE_CONTENT: &str = include_str!("./submit.html");
    let rendered_page = render(PAGE_CONTENT, &[("USER_IP", ip.ip.to_string())]);
    RawHtml(rendered_page)
}

fn render(page_content: &'static str, fields: &[(&str, String)]) -> String {
    let mut page = page_content.to_owned();
    for (field, value) in fields {
        page = page.replace(&format!("<<{field}>>"), value);
    }
    page
}

#[post("/submit", data = "<form>")]
async fn submit(
    form: Form<Strict<UserForm>>,
    ip: IpHeader,
    config: &State<PROpenerConfig>,
) -> Result<Redirect, String> {
    let mut form = form.into_inner().into_inner();
    if form.email.as_ref().is_some_and(|e| e.trim().is_empty()) {
        form.email = None;
    }
    if form.name.as_ref().is_some_and(|n| n.trim().is_empty()) {
        form.name = None;
    }
    let res = match send_pr(form, &ip, config.inner()).await {
        Ok(pr_link) => Ok(Redirect::to(pr_link)),
        Err(err) => Err(format!("{err:#?}")),
    };
    info!("Pr opening outcome: {res:?}");
    res
}

async fn send_pr(form: UserForm, ip: &IpHeader, config: &PROpenerConfig) -> anyhow::Result<String> {
    let payload: Payload = serde_json::from_str(&form.payload)
        .context("Failed to parse payload JSON — expected {\"patch\": ..., \"pr-title\": ..., \"pr-body\": ...}")?;

    info!(
        "PR submitted name={:?} email={:?} ip={} pr_title={:?}",
        form.name, form.email, ip.ip, payload.pr_title
    );
    append_json_lines(
        LOGS_PATH,
        &[PRLog {
            form: &form,
            ip,
            pr_title: &payload.pr_title,
            pr_body: &payload.pr_body,
        }],
    )
    .with_context(|| format!("LOG_FILE={LOGS_PATH} is not writable"))?;

    info!("Fetching latest main...");
    git::fetch_main(&config.repo_path).context("Failed to fetch latest main")?;

    let tmp_dir = TempDir::new("propener-worktree").context("Could not create temp dir")?;
    let worktree_path = tmp_dir.path().join("worktree");
    let branch = format!(
        "pr-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    info!("Creating worktree for branch {branch}...");
    let _worktree = git::create_worktree(&config.repo_path, &worktree_path, &branch)
        .context("Failed to create worktree")?;

    info!("Applying patch...");
    git::apply_patch(&worktree_path, &payload.patch).context("Failed to apply patch")?;

    info!("Amending commit...");
    let worktree_repo = gix::open(&worktree_path).context("Failed to open worktree as repo")?;
    let commit_id = git::amend(
        &worktree_repo,
        form.name.as_deref(),
        form.email.as_deref(),
        |old_message| {
            format!(
                indoc! {"
        Declared name: {:?}
        Declared email: {:?}
        ---
        {}
    "},
                form.name, form.email, old_message
            )
        },
    )
    .context("Failed to amend commit")?;
    info!("Amended commit: {commit_id}");

    info!("Pushing branch {branch}...");
    git::push_branch(&config.repo_path, &branch).context("Failed to push branch")?;

    info!("Opening PR via Forgejo API...");
    let pr_link = open_pr(config, &branch, &payload.pr_title, &payload.pr_body)
        .await
        .context("Failed to open PR")?;
    info!("Opened PR: {pr_link}");
    Ok(pr_link)
}

async fn get_or_create_label(
    api: &Forgejo,
    owner: &str,
    repo: &str,
    name: &str,
) -> anyhow::Result<i64> {
    let (_, labels) = api
        .issue_list_labels(owner, repo, IssueListLabelsQuery { sort: None })
        .await
        .context("Failed to list labels")?;

    if let Some(id) = labels
        .iter()
        .find(|l| l.name.as_deref() == Some(name))
        .and_then(|l| l.id)
    {
        return Ok(id);
    }

    let label = api
        .issue_create_label(
            owner,
            repo,
            CreateLabelOption {
                name: name.to_owned(),
                color: "#0075ca".to_owned(),
                description: None,
                exclusive: None,
                is_archived: None,
            },
        )
        .await
        .context("Failed to create label")?;

    label.id.context("Created label has no ID")
}

async fn open_pr(
    config: &PROpenerConfig,
    branch: &str,
    pr_title: &str,
    pr_body: &str,
) -> anyhow::Result<String> {
    let api = Forgejo::new(
        Auth::Token(&config.forgejo_token),
        config.forgejo_url.parse().context("Invalid forgejo URL")?,
    )?;

    let label_id = get_or_create_label(&api, &config.repo_owner, &config.repo_name, "pr-opener")
        .await
        .context("Failed to get/create label")?;

    let pr = api
        .repo_create_pull_request(
            &config.repo_owner,
            &config.repo_name,
            CreatePullRequestOption {
                title: Some(pr_title.to_owned()),
                head: Some(branch.to_owned()),
                base: Some("main".to_owned()),
                body: Some(pr_body.to_owned()),
                assignee: None,
                assignees: None,
                due_date: None,
                labels: Some(vec![label_id]),
                milestone: None,
            },
        )
        .await
        .context("Forgejo API call failed")?;

    pr.html_url
        .map(|u| u.to_string())
        .context("PR created but no URL returned")
}

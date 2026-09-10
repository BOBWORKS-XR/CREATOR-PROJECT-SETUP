const invoke = window.__TAURI__.core.invoke;

const elements = {
  overall: document.querySelector('#overall-status'),
  projectName: document.querySelector('#project-name'),
  parentFolder: document.querySelector('#parent-folder'),
  browse: document.querySelector('#browse-button'),
  refresh: document.querySelector('#refresh-button'),
  create: document.querySelector('#create-button'),
  openProject: document.querySelector('#open-project-button'),
  requirements: document.querySelector('#requirements'),
  blockers: document.querySelector('#blockers'),
  hub: document.querySelector('#hub-button'),
  unityDownload: document.querySelector('#unity-download-button'),
  platform: document.querySelector('#platform-label'),
  recipe: document.querySelector('#recipe-line'),
  activity: document.querySelector('#activity'),
  activityTitle: document.querySelector('#activity-title'),
  activityMessage: document.querySelector('#activity-message'),
  result: document.querySelector('#result'),
  sdkSource: document.querySelector('#sdk-source-button'),
};

let environment = null;
let createdProject = null;

function requirement(name, ok, value) {
  return `<div class="requirement ${ok ? 'ok' : 'bad'}">
    <span class="status-dot"></span><span class="requirement-name">${name}</span>
    <span class="requirement-value">${value}</span>
  </div>`;
}

function escapeHtml(value) {
  return String(value).replace(/[&<>'"]/g, character => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', "'": '&#39;', '"': '&quot;'
  })[character]);
}

function renderEnvironment(report) {
  environment = report;
  const editor = report.editors.find(item => item.exactRecipe);
  elements.platform.textContent = report.platform;
  if (!elements.parentFolder.value && report.suggestedProjectParent) {
    elements.parentFolder.value = report.suggestedProjectParent;
  }
  elements.recipe.textContent = `Approved recipe: Unity ${report.recipe.editorVersion} · Creator SDK ${report.recipe.creatorSdkVersion} · URP ${report.recipe.urpVersion} · Input ${report.recipe.inputSystemVersion}`;
  elements.requirements.innerHTML = [
    requirement('Unity Hub or Unity CLI', report.hubInstalled || report.unityCliInstalled, report.hubInstalled ? 'Hub detected' : report.unityCliInstalled ? 'CLI detected' : 'Missing'),
    requirement(`Unity ${report.recipe.editorVersion}`, Boolean(editor), editor ? 'Installed' : 'Missing'),
    requirement('Android Build Support', Boolean(editor?.androidPlayer), editor?.androidPlayer ? 'Installed' : 'Missing'),
    requirement('Android SDK, NDK and OpenJDK', Boolean(editor?.androidSdk && editor?.androidNdk && editor?.openJdk), editor?.androidSdk && editor?.androidNdk && editor?.openJdk ? 'Installed' : 'Incomplete'),
    requirement('Windows build support', Boolean(editor?.windowsStandalone), editor?.windowsStandalone ? 'Installed' : 'Missing'),
    requirement('Official 3D URP template', Boolean(editor?.urpTemplate), editor?.urpTemplate ? 'Available' : 'Missing'),
  ].join('');

  elements.overall.className = `overall-status ${report.ready ? 'ready' : 'blocked'}`;
  elements.overall.innerHTML = `<span></span>${report.ready ? 'Ready to create' : 'Setup required'}`;
  elements.create.disabled = !report.ready;
  elements.blockers.classList.toggle('hidden', report.blockers.length === 0);
  elements.blockers.innerHTML = report.blockers.map(item => `<p>${escapeHtml(item)}</p>`).join('');
  elements.hub.classList.toggle('hidden', !report.hubInstalled || report.ready);
  elements.unityDownload.classList.toggle('hidden', report.hubInstalled || report.unityCliInstalled);
}

async function refresh() {
  elements.refresh.disabled = true;
  elements.overall.className = 'overall-status checking';
  elements.overall.innerHTML = '<span></span>Checking';
  try {
    renderEnvironment(await invoke('probe_environment'));
  } catch (error) {
    elements.overall.className = 'overall-status blocked';
    elements.overall.innerHTML = '<span></span>Check failed';
    elements.result.className = 'result';
    elements.result.textContent = String(error);
  } finally {
    elements.refresh.disabled = false;
  }
}

elements.browse.addEventListener('click', async () => {
  const folder = await invoke('pick_parent_folder');
  if (folder) elements.parentFolder.value = folder;
});

elements.refresh.addEventListener('click', refresh);

elements.hub.addEventListener('click', async () => {
  await invoke('launch_hub');
});

elements.unityDownload.addEventListener('click', () => invoke('open_official_url', { url: 'https://unity.com/download' }));
elements.sdkSource.addEventListener('click', () => invoke('open_official_url', { url: 'https://greenfield-registry.sdq.st/-/web/detail/com.sidequest.creator-sdk' }));

elements.create.addEventListener('click', async () => {
  elements.create.disabled = true;
  elements.result.classList.add('hidden');
  elements.openProject.classList.add('hidden');
  elements.activity.classList.remove('hidden');
  elements.activityTitle.textContent = 'Creating and validating';
  elements.activityMessage.textContent = 'Unity is resolving packages and compiling. This can take several minutes.';
  try {
    const result = await invoke('create_project', { request: {
      projectName: elements.projectName.value,
      parentDirectory: elements.parentFolder.value,
    }});
    createdProject = result.projectPath;
    elements.result.className = 'result success';
    elements.result.innerHTML = `<strong>Ready</strong><br>${escapeHtml(result.message)}<br><span>${escapeHtml(result.projectPath)}</span>`;
    elements.openProject.classList.remove('hidden');
  } catch (error) {
    elements.result.className = 'result';
    elements.result.innerHTML = `<strong>Setup stopped</strong><br>${escapeHtml(error)}`;
  } finally {
    elements.activity.classList.add('hidden');
    elements.result.classList.remove('hidden');
    elements.create.disabled = !environment?.ready;
  }
});

elements.openProject.addEventListener('click', async () => {
  if (createdProject) await invoke('open_project', { path: createdProject });
});

refresh();

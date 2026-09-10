const invoke = window.__TAURI__.core.invoke;

const elements = {
  overall: document.querySelector('#overall-status'),
  projectName: document.querySelector('#project-name'),
  parentFolder: document.querySelector('#parent-folder'),
  browse: document.querySelector('#browse-button'),
  refresh: document.querySelector('#refresh-button'),
  create: document.querySelector('#create-button'),
  openProject: document.querySelector('#open-project-button'),
  createAnother: document.querySelector('#create-another-button'),
  actionError: document.querySelector('#action-error'),
  requirements: document.querySelector('#requirements'),
  blockers: document.querySelector('#blockers'),
  hub: document.querySelector('#hub-button'),
  unityDownload: document.querySelector('#unity-download-button'),
  platform: document.querySelector('#platform-label'),
  recipe: document.querySelector('#recipe-line'),
  activity: document.querySelector('#activity'),
  activityTitle: document.querySelector('#activity-title'),
  activityMessage: document.querySelector('#activity-message'),
  progressSteps: document.querySelector('#progress-steps'),
  elapsed: document.querySelector('#elapsed-time'),
  result: document.querySelector('#result'),
  sdkSource: document.querySelector('#sdk-source-button'),
};

let environment = null;
let createdProject = null;
let busy = false;
let checking = false;
let opening = false;
const stages = ['Check requirements', 'Prepare project', 'Import and compile', 'Configure Visual Scripting', 'Reopen and validate', 'Project ready'];
let currentStep = 0;

function renderProgress({ step, detail }) {
  if (!Number.isInteger(step) || step < currentStep || step < 1 || step > stages.length) return;
  currentStep = step;
  elements.activityTitle.textContent = stages[step - 1];
  elements.activityMessage.textContent = detail;
  elements.progressSteps.innerHTML = stages.map((label, index) => `<li class="${index + 1 < step ? 'done' : index + 1 === step ? 'current' : ''}" ${index + 1 === step ? 'aria-current="step"' : ''}>${label}</li>`).join('');
}

function updateControls() {
  const locked = busy || checking || Boolean(createdProject);
  for (const field of [elements.projectName, elements.parentFolder, elements.browse]) field.disabled = locked;
  elements.create.disabled = locked || !environment?.ready;
  elements.create.classList.toggle('hidden', Boolean(createdProject));
  elements.openProject.classList.toggle('hidden', !createdProject);
  elements.openProject.disabled = opening;
  elements.createAnother.classList.toggle('hidden', !createdProject);
  elements.createAnother.disabled = opening || checking;
  elements.refresh.disabled = busy || checking;
  const label = busy ? 'Creating project' : createdProject ? 'Project ready' : checking ? 'Checking' : environment?.ready ? 'Ready to create' : 'Setup required';
  elements.overall.className = `overall-status ${createdProject || (!busy && !checking && environment?.ready) ? 'ready' : 'checking'}`;
  elements.overall.innerHTML = `<span></span>${label}`;
}

function showActionError(error) {
  elements.actionError.textContent = String(error);
  elements.actionError.classList.remove('hidden');
}

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

  elements.blockers.classList.toggle('hidden', report.blockers.length === 0);
  elements.blockers.innerHTML = report.blockers.map(item => `<p>${escapeHtml(item)}</p>`).join('');
  elements.hub.classList.toggle('hidden', !report.hubInstalled || report.ready);
  elements.unityDownload.classList.toggle('hidden', report.hubInstalled || report.unityCliInstalled);
}

async function refresh() {
  if (busy || checking) return;
  checking = true;
  updateControls();
  try {
    renderEnvironment(await invoke('probe_environment'));
  } catch (error) {
    environment = null;
    showActionError(error);
  } finally {
    checking = false;
    updateControls();
  }
}

elements.browse.addEventListener('click', async () => {
  if (busy || checking || createdProject) return;
  try {
    const folder = await invoke('pick_parent_folder');
    if (folder && !busy && !createdProject) elements.parentFolder.value = folder;
  } catch (error) { showActionError(error); }
});

elements.refresh.addEventListener('click', refresh);

elements.hub.addEventListener('click', async () => {
  try { await invoke('launch_hub'); } catch (error) { showActionError(error); }
});

elements.unityDownload.addEventListener('click', () => invoke('open_official_url', { url: 'https://unity.com/download' }).catch(showActionError));
elements.sdkSource.addEventListener('click', () => invoke('open_official_url', { url: 'https://greenfield-registry.sdq.st/-/web/detail/com.sidequest.creator-sdk' }).catch(showActionError));

elements.create.addEventListener('click', async () => {
  if (busy || checking || createdProject || !environment?.ready) return;
  busy = true;
  updateControls();
  elements.actionError.classList.add('hidden');
  elements.result.classList.add('hidden');
  elements.openProject.classList.add('hidden');
  elements.activity.classList.remove('hidden');
  currentStep = 0;
  renderProgress({ step: 1, detail: 'Checking project path and Unity requirements.' });
  const started = Date.now();
  const updateElapsed = () => {
    const seconds = Math.floor((Date.now() - started) / 1000);
    elements.elapsed.textContent = `${Math.floor(seconds / 60)}m ${String(seconds % 60).padStart(2, '0')}s elapsed`;
  };
  updateElapsed();
  const timer = setInterval(updateElapsed, 1000);
  let unlisten;
  try {
    unlisten = await window.__TAURI__.event.listen('setup-progress', event => renderProgress(event.payload));
    const result = await invoke('create_project', { request: {
      projectName: elements.projectName.value,
      parentDirectory: elements.parentFolder.value,
    }});
    if (!result.success) throw new Error(result.message || 'Project validation failed.');
    createdProject = result.projectPath;
    elements.result.className = 'result success';
    elements.result.innerHTML = `<strong>Ready</strong><br>${escapeHtml(result.message)}<br><span>${escapeHtml(result.projectPath)}</span>`;
    elements.openProject.classList.remove('hidden');
  } catch (error) {
    elements.result.className = 'result';
    elements.result.innerHTML = `<strong>Setup stopped</strong><br>${escapeHtml(error)}`;
  } finally {
    clearInterval(timer);
    if (unlisten) unlisten();
    busy = false;
    elements.activity.classList.add('hidden');
    elements.result.classList.remove('hidden');
    updateControls();
  }
});

elements.openProject.addEventListener('click', async () => {
  if (!createdProject || opening) return;
  opening = true;
  elements.actionError.classList.add('hidden');
  updateControls();
  try { await invoke('open_project', { path: createdProject }); }
  catch (error) { showActionError(error); }
  finally { opening = false; updateControls(); }
});

elements.createAnother.addEventListener('click', () => {
  if (opening || checking || busy) return;
  createdProject = null;
  elements.projectName.value = '';
  elements.result.classList.add('hidden');
  elements.actionError.classList.add('hidden');
  updateControls();
  elements.projectName.focus();
});

refresh();

//! Embedded HTML for the Studio visual workflow builder.

/// Returns the embedded Studio HTML page.
pub fn studio_html() -> &'static str {
    r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OneCrawl Studio</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0d1117; color: #c9d1d9; }
        .header { background: #161b22; border-bottom: 1px solid #30363d; padding: 12px 24px; display: flex; align-items: center; justify-content: space-between; }
        .header h1 { font-size: 20px; color: #58a6ff; }
        .header .version { color: #8b949e; font-size: 13px; }
        .main { display: flex; height: calc(100vh - 92px); }
        .sidebar { width: 280px; background: #161b22; border-right: 1px solid #30363d; overflow-y: auto; }
        .sidebar-section { padding: 16px; border-bottom: 1px solid #30363d; }
        .sidebar-section h3 { font-size: 12px; text-transform: uppercase; color: #8b949e; margin-bottom: 8px; letter-spacing: 0.5px; }
        .canvas { flex: 1; padding: 24px; overflow-y: auto; }
        .properties { width: 320px; background: #161b22; border-left: 1px solid #30363d; overflow-y: auto; padding: 16px; }
        .btn { padding: 6px 16px; border-radius: 6px; border: 1px solid #30363d; background: #21262d; color: #c9d1d9; cursor: pointer; font-size: 13px; transition: all 0.15s; }
        .btn:hover { background: #30363d; border-color: #8b949e; }
        .btn-primary { background: #238636; border-color: #238636; color: white; }
        .btn-primary:hover { background: #2ea043; }
        .btn-danger { background: #da3633; border-color: #da3633; color: white; }
        .btn-sm { padding: 4px 12px; font-size: 12px; }
        .template-card { background: #21262d; border: 1px solid #30363d; border-radius: 8px; padding: 12px; margin-bottom: 8px; cursor: pointer; transition: all 0.15s; }
        .template-card:hover { border-color: #58a6ff; background: #1c2128; }
        .template-card h4 { font-size: 14px; color: #e6edf3; margin-bottom: 4px; }
        .template-card p { font-size: 12px; color: #8b949e; }
        .template-card .tags { margin-top: 6px; }
        .tag { display: inline-block; padding: 2px 8px; border-radius: 12px; background: #1f6feb33; color: #58a6ff; font-size: 11px; margin-right: 4px; }
        .step-block { background: #21262d; border: 1px solid #30363d; border-radius: 8px; padding: 16px; margin-bottom: 12px; position: relative; transition: all 0.15s; }
        .step-block:hover { border-color: #58a6ff; }
        .step-block.selected { border-color: #58a6ff; box-shadow: 0 0 0 1px #58a6ff; }
        .step-block .step-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
        .step-block .step-number { background: #30363d; color: #8b949e; padding: 2px 8px; border-radius: 10px; font-size: 11px; font-weight: bold; }
        .step-block .step-type { color: #7ee787; font-size: 12px; font-weight: 500; }
        .step-block .step-name { font-size: 14px; color: #e6edf3; }
        .step-block .step-detail { font-size: 12px; color: #8b949e; margin-top: 4px; }
        .step-block .step-actions { position: absolute; top: 8px; right: 8px; display: none; gap: 4px; }
        .step-block:hover .step-actions { display: flex; }
        .add-step { border: 2px dashed #30363d; border-radius: 8px; padding: 16px; text-align: center; color: #8b949e; cursor: pointer; transition: all 0.15s; }
        .add-step:hover { border-color: #58a6ff; color: #58a6ff; }
        .prop-group { margin-bottom: 16px; }
        .prop-group label { display: block; font-size: 12px; color: #8b949e; margin-bottom: 4px; }
        .prop-group input, .prop-group select, .prop-group textarea { width: 100%; padding: 8px; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; color: #c9d1d9; font-size: 13px; }
        .prop-group input:focus, .prop-group select:focus, .prop-group textarea:focus { border-color: #58a6ff; outline: none; }
        .prop-group textarea { min-height: 80px; resize: vertical; font-family: monospace; }
        .json-editor { background: #0d1117; border: 1px solid #30363d; border-radius: 8px; padding: 16px; font-family: monospace; font-size: 13px; min-height: 200px; white-space: pre; overflow-x: auto; }
        .toolbar { display: flex; gap: 8px; padding: 12px 24px; background: #161b22; border-bottom: 1px solid #30363d; }
        .project-item { display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; border-radius: 6px; cursor: pointer; transition: all 0.15s; }
        .project-item:hover { background: #21262d; }
        .project-item .name { font-size: 13px; color: #e6edf3; }
        .project-item .date { font-size: 11px; color: #8b949e; }
        .status-bar { background: #161b22; border-top: 1px solid #30363d; padding: 4px 24px; font-size: 11px; color: #8b949e; display: flex; justify-content: space-between; }
        .empty-state { text-align: center; padding: 60px 24px; color: #8b949e; }
        .empty-state h2 { font-size: 24px; color: #e6edf3; margin-bottom: 8px; }
        .empty-state p { margin-bottom: 24px; }
        .modal-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: none; align-items: center; justify-content: center; z-index: 1000; }
        .modal-overlay.active { display: flex; }
        .modal { background: #161b22; border: 1px solid #30363d; border-radius: 12px; padding: 24px; min-width: 400px; max-width: 600px; }
        .modal h2 { font-size: 18px; color: #e6edf3; margin-bottom: 16px; }
    </style>
</head>
<body>
    <div class="header">
        <div style="display:flex;align-items:center;gap:12px;">
            <h1>&#x1f577;&#xfe0f; OneCrawl Studio</h1>
            <span class="version">Visual Workflow Builder</span>
        </div>
        <div style="display:flex;gap:8px;">
            <button class="btn" onclick="showTemplates()">&#x1f4cb; Templates</button>
            <button class="btn" onclick="importWorkflow()">&#x1f4e5; Import</button>
            <button class="btn btn-primary" onclick="exportWorkflow()">&#x1f4e4; Export</button>
        </div>
    </div>
    <div class="toolbar">
        <button class="btn btn-sm" onclick="newProject()">&#x2795; New</button>
        <button class="btn btn-sm" onclick="saveProject()">&#x1f4be; Save</button>
        <button class="btn btn-sm" onclick="validateWorkflow()">&#x2705; Validate</button>
        <button class="btn btn-sm btn-primary" onclick="runWorkflow()">&#x25b6;&#xfe0f; Run</button>
        <div style="flex:1"></div>
        <button class="btn btn-sm" onclick="toggleJson()">{ } JSON</button>
    </div>
    <div class="main">
        <div class="sidebar">
            <div class="sidebar-section">
                <h3>Projects</h3>
                <div id="projects-list"></div>
            </div>
            <div class="sidebar-section">
                <h3>Action Palette</h3>
                <div id="action-palette">
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'navigate')"><h4>&#x1f310; Navigate</h4><p>Go to URL</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'click')"><h4>&#x1f446; Click</h4><p>Click element</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'type')"><h4>&#x2328;&#xfe0f; Type</h4><p>Type text</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'smart_click')"><h4>&#x1f3af; Smart Click</h4><p>AI-powered click</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'smart_fill')"><h4>&#x2728; Smart Fill</h4><p>AI-powered form fill</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'extract')"><h4>&#x1f4e6; Extract</h4><p>Extract data</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'screenshot')"><h4>&#x1f4f8; Screenshot</h4><p>Capture page</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'wait_for_selector')"><h4>&#x23f3; Wait</h4><p>Wait for element</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'evaluate')"><h4>&#x1f4bb; Evaluate</h4><p>Run JavaScript</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'assert')"><h4>&#x2713; Assert</h4><p>Verify condition</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'sleep')"><h4>&#x1f4a4; Sleep</h4><p>Wait (ms)</p></div>
                    <div class="template-card" draggable="true" ondragstart="dragStep(event,'log')"><h4>&#x1f4dd; Log</h4><p>Log message</p></div>
                </div>
            </div>
        </div>
        <div class="canvas" id="canvas" ondragover="event.preventDefault()" ondrop="dropStep(event)">
            <div id="workflow-steps"></div>
            <div class="add-step" onclick="addStep()">+ Add Step</div>
        </div>
        <div class="properties" id="properties">
            <h3 style="font-size:14px;margin-bottom:16px;color:#e6edf3;">Properties</h3>
            <div id="prop-content">
                <p style="color:#8b949e;font-size:13px;">Select a step to edit its properties</p>
            </div>
        </div>
    </div>
    <div class="status-bar">
        <span id="status-text">Ready</span>
        <span id="step-count">0 steps</span>
    </div>

    <div class="modal-overlay" id="templates-modal">
        <div class="modal">
            <h2>Workflow Templates</h2>
            <div id="templates-list"></div>
            <div style="margin-top:16px;text-align:right;">
                <button class="btn" onclick="closeModal('templates-modal')">Close</button>
            </div>
        </div>
    </div>

    <script>
    var currentProject = null;
    var steps = [];
    var selectedStep = -1;
    var API_BASE = window.location.origin;

    var STEP_TYPES = {
        navigate: { icon: '\u{1f310}', label: 'Navigate', fields: [{name:'url',type:'text',label:'URL',required:true}] },
        click: { icon: '\u{1f446}', label: 'Click', fields: [{name:'selector',type:'text',label:'CSS Selector',required:true}] },
        type: { icon: '\u{2328}\u{fe0f}', label: 'Type', fields: [{name:'selector',type:'text',label:'Selector',required:true},{name:'text',type:'text',label:'Text',required:true}] },
        smart_click: { icon: '\u{1f3af}', label: 'Smart Click', fields: [{name:'query',type:'text',label:'Query',required:true}] },
        smart_fill: { icon: '\u{2728}', label: 'Smart Fill', fields: [{name:'query',type:'text',label:'Query',required:true},{name:'value',type:'text',label:'Value',required:true}] },
        extract: { icon: '\u{1f4e6}', label: 'Extract', fields: [{name:'selector',type:'text',label:'Selector',required:true},{name:'attributes',type:'text',label:'Attributes (comma-sep)'}] },
        screenshot: { icon: '\u{1f4f8}', label: 'Screenshot', fields: [{name:'path',type:'text',label:'Save Path'}] },
        wait_for_selector: { icon: '\u{23f3}', label: 'Wait', fields: [{name:'selector',type:'text',label:'Selector',required:true},{name:'timeout',type:'number',label:'Timeout (ms)',default:'10000'}] },
        evaluate: { icon: '\u{1f4bb}', label: 'Evaluate', fields: [{name:'script',type:'textarea',label:'JavaScript',required:true}] },
        assert: { icon: '\u{2713}', label: 'Assert', fields: [{name:'condition',type:'text',label:'Condition',required:true},{name:'value',type:'text',label:'Expected Value'}] },
        sleep: { icon: '\u{1f4a4}', label: 'Sleep', fields: [{name:'ms',type:'number',label:'Duration (ms)',required:true,default:'1000'}] },
        log: { icon: '\u{1f4dd}', label: 'Log', fields: [{name:'message',type:'text',label:'Message',required:true}] }
    };

    function escapeHtml(s) { return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }

    function renderSteps() {
        var container = document.getElementById('workflow-steps');
        container.innerHTML = steps.map(function(step, i) {
            var meta = STEP_TYPES[step.action.type] || { icon: '?', label: step.action.type };
            var detail = step.action.url || step.action.selector || step.action.query || step.action.text || (step.action.script ? step.action.script.substring(0,40) : '') || step.action.message || '';
            return '<div class="step-block' + (i === selectedStep ? ' selected' : '') + '" onclick="selectStep(' + i + ')" draggable="true" ondragstart="dragReorder(event,' + i + ')" ondragover="event.preventDefault()" ondrop="dropReorder(event,' + i + ')">' +
                '<div class="step-header"><span class="step-number">#' + (i+1) + '</span><span class="step-type">' + meta.icon + ' ' + meta.label + '</span></div>' +
                '<div class="step-name">' + escapeHtml(step.name || 'Step ' + (i+1)) + '</div>' +
                (detail ? '<div class="step-detail">' + escapeHtml(String(detail)) + '</div>' : '') +
                '<div class="step-actions"><button class="btn btn-sm" onclick="event.stopPropagation();moveStep(' + i + ',-1)">\u2191</button><button class="btn btn-sm" onclick="event.stopPropagation();moveStep(' + i + ',1)">\u2193</button><button class="btn btn-sm btn-danger" onclick="event.stopPropagation();removeStep(' + i + ')">\u2715</button></div>' +
                '</div>';
        }).join('');
        document.getElementById('step-count').textContent = steps.length + ' steps';
    }

    function selectStep(i) { selectedStep = i; renderSteps(); renderProperties(); }

    function renderProperties() {
        var container = document.getElementById('prop-content');
        if (selectedStep < 0 || selectedStep >= steps.length) {
            container.innerHTML = '<p style="color:#8b949e;font-size:13px;">Select a step to edit</p>';
            return;
        }
        var step = steps[selectedStep];
        var meta = STEP_TYPES[step.action.type] || { fields: [] };
        var html = '<div class="prop-group"><label>Step Name</label><input type="text" value="' + escapeHtml(step.name || '') + '" onchange="updateStepName(this.value)"></div>';
        html += '<div class="prop-group"><label>Action Type</label><select onchange="changeActionType(this.value)">';
        for (var key in STEP_TYPES) {
            html += '<option value="' + key + '"' + (key === step.action.type ? ' selected' : '') + '>' + STEP_TYPES[key].icon + ' ' + STEP_TYPES[key].label + '</option>';
        }
        html += '</select></div>';
        meta.fields.forEach(function(field) {
            var val = step.action[field.name] || field.default || '';
            if (field.type === 'textarea') {
                html += '<div class="prop-group"><label>' + field.label + (field.required ? ' *' : '') + '</label><textarea onchange="updateField(\'' + field.name + '\',this.value)">' + escapeHtml(String(val)) + '</textarea></div>';
            } else {
                html += '<div class="prop-group"><label>' + field.label + (field.required ? ' *' : '') + '</label><input type="' + field.type + '" value="' + escapeHtml(String(val)) + '" onchange="updateField(\'' + field.name + '\',this.value)"></div>';
            }
        });
        html += '<div class="prop-group"><label>Save As Variable</label><input type="text" value="' + escapeHtml(step.save_as || '') + '" onchange="updateSaveAs(this.value)"></div>';
        html += '<div class="prop-group"><label>Condition (skip if)</label><input type="text" value="' + escapeHtml(step.condition || '') + '" onchange="updateCondition(this.value)"></div>';
        container.innerHTML = html;
    }

    function updateStepName(v) { steps[selectedStep].name = v; renderSteps(); }
    function updateField(f, v) { steps[selectedStep].action[f] = v; renderSteps(); }
    function updateSaveAs(v) { steps[selectedStep].save_as = v || undefined; }
    function updateCondition(v) { steps[selectedStep].condition = v || undefined; }
    function changeActionType(t) { steps[selectedStep].action = { type: t }; renderSteps(); renderProperties(); }

    function addStep(type) {
        type = type || 'navigate';
        steps.push({ name: '', action: { type: type }, save_as: undefined, condition: undefined });
        selectedStep = steps.length - 1;
        renderSteps(); renderProperties();
    }

    function removeStep(i) {
        steps.splice(i, 1);
        if (selectedStep >= steps.length) selectedStep = steps.length - 1;
        renderSteps(); renderProperties();
    }

    function moveStep(i, dir) {
        var j = i + dir;
        if (j < 0 || j >= steps.length) return;
        var tmp = steps[i]; steps[i] = steps[j]; steps[j] = tmp;
        selectedStep = j;
        renderSteps(); renderProperties();
    }

    function dragStep(e, type) { e.dataTransfer.setData('newStep', type); }
    function dropStep(e) { e.preventDefault(); var type = e.dataTransfer.getData('newStep'); if (type) addStep(type); }

    var dragIndex = -1;
    function dragReorder(e, i) { dragIndex = i; e.dataTransfer.setData('reorder', '1'); }
    function dropReorder(e, i) {
        e.preventDefault();
        if (e.dataTransfer.getData('reorder') && dragIndex >= 0 && dragIndex !== i) {
            var item = steps.splice(dragIndex, 1)[0];
            steps.splice(i, 0, item);
            selectedStep = i;
            renderSteps(); renderProperties();
        }
    }

    function buildWorkflow() {
        return { name: (currentProject && currentProject.name) || 'Untitled', steps: steps.map(function(s) {
            var step = { name: s.name, action: Object.assign({}, s.action) };
            if (s.save_as) step.save_as = s.save_as;
            if (s.condition) step.condition = s.condition;
            return step;
        })};
    }

    function exportWorkflow() {
        var wf = JSON.stringify(buildWorkflow(), null, 2);
        var blob = new Blob([wf], { type: 'application/json' });
        var a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = ((currentProject && currentProject.name) || 'workflow') + '.json';
        a.click();
        setStatus('Exported workflow');
    }

    function importWorkflow() {
        var input = document.createElement('input');
        input.type = 'file'; input.accept = '.json';
        input.onchange = function(e) {
            var file = e.target.files[0];
            if (!file) return;
            var reader = new FileReader();
            reader.onload = function(re) {
                try {
                    var wf = JSON.parse(re.target.result);
                    steps = (wf.steps || []).map(function(s) { return { name: s.name || '', action: s.action || { type: 'navigate' }, save_as: s.save_as, condition: s.condition }; });
                    currentProject = { id: Date.now().toString(), name: wf.name || file.name.replace('.json','') };
                    renderSteps();
                    setStatus('Imported: ' + file.name);
                } catch(err) { alert('Invalid JSON: ' + err.message); }
            };
            reader.readAsText(file);
        };
        input.click();
    }

    function showTemplates() {
        fetch(API_BASE + '/studio/api/templates').then(function(r) { return r.json(); }).then(function(templates) {
            var list = document.getElementById('templates-list');
            list.innerHTML = templates.map(function(t) {
                return '<div class="template-card" onclick="loadTemplate(\'' + t.id + '\')">' +
                '<h4>' + escapeHtml(t.name) + '</h4><p>' + escapeHtml(t.description) + '</p>' +
                '<div class="tags">' + t.tags.map(function(tag) { return '<span class="tag">' + escapeHtml(tag) + '</span>'; }).join('') + '</div></div>';
            }).join('');
            document.getElementById('templates-modal').classList.add('active');
        }).catch(function() { alert('Failed to load templates'); });
    }

    function loadTemplate(id) {
        fetch(API_BASE + '/studio/api/templates/' + id).then(function(r) { return r.json(); }).then(function(t) {
            var wf = t.workflow;
            steps = (wf.steps || []).map(function(s) { return { name: s.name || '', action: s.action || { type: 'navigate' }, save_as: s.save_as, condition: s.condition }; });
            currentProject = { id: Date.now().toString(), name: t.name };
            renderSteps(); closeModal('templates-modal');
            setStatus('Loaded template: ' + t.name);
        });
    }

    function newProject() {
        var name = prompt('Project name:');
        if (!name) return;
        steps = []; selectedStep = -1;
        currentProject = { id: Date.now().toString(), name: name };
        renderSteps(); renderProperties();
        setStatus('New project: ' + name);
    }

    function saveProject() {
        if (!currentProject) { alert('No project open'); return; }
        var project = { id: currentProject.id, name: currentProject.name, workflow: buildWorkflow() };
        fetch(API_BASE + '/studio/api/projects', {
            method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(project)
        }).then(function(res) {
            if (res.ok) { setStatus('Saved: ' + currentProject.name); loadProjects(); }
            else { alert('Save failed'); }
        }).catch(function(e) { alert('Save error: ' + e.message); });
    }

    function loadProjects() {
        fetch(API_BASE + '/studio/api/projects').then(function(r) { return r.json(); }).then(function(projects) {
            var list = document.getElementById('projects-list');
            list.innerHTML = projects.map(function(p) {
                return '<div class="project-item" onclick="openProject(\'' + escapeHtml(p.id) + '\')">' +
                '<span class="name">' + escapeHtml(p.name) + '</span>' +
                '<span class="date">' + (p.updated_at || '').substring(0,10) + '</span></div>';
            }).join('') || '<p style="color:#8b949e;font-size:12px;padding:8px;">No projects yet</p>';
        }).catch(function() {});
    }

    function openProject(id) {
        fetch(API_BASE + '/studio/api/projects/' + encodeURIComponent(id)).then(function(r) { return r.json(); }).then(function(project) {
            currentProject = project;
            var wf = project.workflow || {};
            steps = (wf.steps || []).map(function(s) { return { name: s.name || '', action: s.action || { type: 'navigate' }, save_as: s.save_as, condition: s.condition }; });
            selectedStep = -1;
            renderSteps(); renderProperties();
            setStatus('Opened: ' + project.name);
        }).catch(function() { alert('Failed to open project'); });
    }

    function validateWorkflow() {
        fetch(API_BASE + '/studio/api/validate', {
            method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(buildWorkflow())
        }).then(function(r) { return r.json(); }).then(function(result) {
            if (result.warnings && result.warnings.length > 0) {
                alert('Warnings:\n' + result.warnings.join('\n'));
            } else { setStatus('\u2705 Workflow is valid'); }
        }).catch(function(e) { alert('Validation error: ' + e.message); });
    }

    function runWorkflow() {
        if (steps.length === 0) { alert('No steps to run'); return; }
        setStatus('\u25b6\ufe0f Running workflow...');
        alert('Workflow execution requires an active browser tab.\nUse: POST /studio/api/run with the workflow JSON.');
        setStatus('Ready');
    }

    var showingJson = false;
    function toggleJson() {
        showingJson = !showingJson;
        if (showingJson) {
            document.getElementById('canvas').innerHTML = '<div class="json-editor">' + escapeHtml(JSON.stringify(buildWorkflow(), null, 2)) + '</div>';
        } else {
            document.getElementById('canvas').innerHTML = '<div id="workflow-steps"></div><div class="add-step" onclick="addStep()">+ Add Step</div>';
            renderSteps();
        }
    }

    function closeModal(id) { document.getElementById(id).classList.remove('active'); }
    function setStatus(text) { document.getElementById('status-text').textContent = text; }

    loadProjects();
    renderSteps();
    </script>
</body>
</html>"##
}

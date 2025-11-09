// API Base URL
const API_BASE = '/api';

// State
let courses = [];
let statusInterval = null;

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    initializeUI();
    loadConfiguration();
    startStatusPolling();
    setupModeToggle();
});

// Setup monitoring mode toggle
function setupModeToggle() {
    const modeSelect = document.getElementById('monitoring-mode');
    const thresholdGroup = document.getElementById('threshold-group');

    modeSelect.addEventListener('change', (e) => {
        if (e.target.value === 'include') {
            thresholdGroup.style.display = 'none';
        } else {
            thresholdGroup.style.display = 'block';
        }
    });

    // Initial setup
    if (modeSelect.value === 'include') {
        thresholdGroup.style.display = 'none';
    }
}

// Initialize UI with one default course
function initializeUI() {
    if (courses.length === 0) {
        addCourse();
    }
}

// Load configuration from server
async function loadConfiguration() {
    try {
        const response = await fetch(`${API_BASE}/config`);
        if (!response.ok) throw new Error('Failed to load configuration');

        const config = await response.json();

        // Populate form fields
        document.getElementById('term').value = config.term || '';
        document.getElementById('polling-interval').value = config.polling_interval || 30;
        document.getElementById('monitoring-mode').value = config.monitoring_mode || 'include';
        document.getElementById('seat-threshold').value = config.seat_threshold || 3;

        // Trigger mode change event
        document.getElementById('monitoring-mode').dispatchEvent(new Event('change'));

    } catch (error) {
        console.error('Error loading configuration:', error);
    }
}

// Add a new course
function addCourse() {
    const courseId = `course-${Date.now()}`;
    const course = {
        id: courseId,
        department: '',
        course_code: '',
        sections: []
    };

    courses.push(course);
    renderCourses();
}

// Remove a course
function removeCourse(courseId) {
    courses = courses.filter(c => c.id !== courseId);
    renderCourses();
}

// Add section group to course
function addSectionGroup(courseId) {
    const course = courses.find(c => c.id === courseId);
    if (course) {
        course.sections.push({
            lecture: '',
            discussions: []
        });
        renderCourses();
    }
}

// Remove section group from course
function removeSectionGroup(courseId, sectionIndex) {
    const course = courses.find(c => c.id === courseId);
    if (course) {
        course.sections.splice(sectionIndex, 1);
        renderCourses();
    }
}

// Add discussion section
function addDiscussion(courseId, sectionIndex) {
    const course = courses.find(c => c.id === courseId);
    if (course && course.sections[sectionIndex]) {
        course.sections[sectionIndex].discussions.push('');
        renderCourses();
    }
}

// Remove discussion section
function removeDiscussion(courseId, sectionIndex, discussionIndex) {
    const course = courses.find(c => c.id === courseId);
    if (course && course.sections[sectionIndex]) {
        course.sections[sectionIndex].discussions.splice(discussionIndex, 1);
        renderCourses();
    }
}

// Update course field
function updateCourseField(courseId, field, value) {
    const course = courses.find(c => c.id === courseId);
    if (course) {
        course[field] = value;
    }
}

// Update section field
function updateSectionField(courseId, sectionIndex, field, value) {
    const course = courses.find(c => c.id === courseId);
    if (course && course.sections[sectionIndex]) {
        course.sections[sectionIndex][field] = value;
    }
}

// Update discussion field
function updateDiscussionField(courseId, sectionIndex, discussionIndex, value) {
    const course = courses.find(c => c.id === courseId);
    if (course && course.sections[sectionIndex]) {
        course.sections[sectionIndex].discussions[discussionIndex] = value;
    }
}

// Render courses
function renderCourses() {
    const container = document.getElementById('courses-container');
    container.innerHTML = '';

    courses.forEach((course, courseIndex) => {
        const courseCard = document.createElement('div');
        courseCard.className = 'course-card';

        courseCard.innerHTML = `
            <div class="course-header">
                <div class="course-title">Course ${courseIndex + 1}</div>
                <button class="remove-course" onclick="removeCourse('${course.id}')">Remove Course</button>
            </div>

            <div class="course-grid">
                <div class="form-group">
                    <label>Department</label>
                    <input type="text"
                           value="${course.department}"
                           onchange="updateCourseField('${course.id}', 'department', this.value)"
                           placeholder="e.g., CHEM, CSE, BILD">
                </div>
                <div class="form-group">
                    <label>Course Code</label>
                    <input type="text"
                           value="${course.course_code}"
                           onchange="updateCourseField('${course.id}', 'course_code', this.value)"
                           placeholder="e.g., 6B, 100, 1">
                </div>
            </div>

            <div class="sections-container">
                <h4 style="margin-bottom: 15px; color: var(--secondary-color);">Section Groups</h4>
                ${renderSectionGroups(course)}
                <button type="button" class="btn btn-small btn-secondary" onclick="addSectionGroup('${course.id}')">
                    + Add Section Group
                </button>
            </div>
        `;

        container.appendChild(courseCard);
    });
}

// Render section groups for a course
function renderSectionGroups(course) {
    if (!course.sections || course.sections.length === 0) {
        return '<p style="color: #6c757d; margin-bottom: 15px;">No section groups added yet.</p>';
    }

    return course.sections.map((section, sectionIndex) => `
        <div class="section-group">
            <div class="section-group-header">
                <div class="section-group-title">Section Group ${sectionIndex + 1}</div>
                <button class="remove-section-group" onclick="removeSectionGroup('${course.id}', ${sectionIndex})">
                    Remove
                </button>
            </div>

            <div class="form-group">
                <label>Lecture Section</label>
                <input type="text"
                       value="${section.lecture}"
                       onchange="updateSectionField('${course.id}', ${sectionIndex}, 'lecture', this.value)"
                       placeholder="e.g., A00, B00, C00">
            </div>

            <div class="discussions-container">
                <label>Discussion Sections</label>
                ${renderDiscussions(course.id, sectionIndex, section.discussions)}
                <button type="button" class="btn btn-small btn-secondary"
                        onclick="addDiscussion('${course.id}', ${sectionIndex})">
                    + Add Discussion
                </button>
            </div>
        </div>
    `).join('');
}

// Render discussion sections
function renderDiscussions(courseId, sectionIndex, discussions) {
    if (!discussions || discussions.length === 0) {
        return '<p style="color: #6c757d; font-size: 0.9rem; margin-bottom: 10px;">No discussions added.</p>';
    }

    return discussions.map((discussion, discussionIndex) => `
        <div class="discussion-item">
            <input type="text"
                   value="${discussion}"
                   onchange="updateDiscussionField('${courseId}', ${sectionIndex}, ${discussionIndex}, this.value)"
                   placeholder="e.g., A01, A02, A03">
            <button onclick="removeDiscussion('${courseId}', ${sectionIndex}, ${discussionIndex})">
                Remove
            </button>
        </div>
    `).join('');
}

// Save configuration
async function saveConfiguration() {
    try {
        // Validate required fields
        const term = document.getElementById('term').value.trim();
        const cookie = document.getElementById('cookie').value.trim();
        const pollingInterval = parseInt(document.getElementById('polling-interval').value);
        const monitoringMode = document.getElementById('monitoring-mode').value;
        const seatThreshold = parseInt(document.getElementById('seat-threshold').value);

        if (!term || !cookie) {
            showMessage('Please fill in all required fields (Term and Cookie)', 'error');
            return;
        }

        if (courses.length === 0) {
            showMessage('Please add at least one course', 'error');
            return;
        }

        // Validate courses
        for (const course of courses) {
            if (!course.department || !course.course_code) {
                showMessage('Please fill in department and course code for all courses', 'error');
                return;
            }
        }

        const config = {
            term,
            polling_interval: pollingInterval,
            cookie,
            courses: courses.map(c => ({
                department: c.department,
                course_code: c.course_code,
                sections: c.sections || []
            })),
            seat_threshold: seatThreshold,
            monitoring_mode: monitoringMode
        };

        const response = await fetch(`${API_BASE}/jobs`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(config)
        });

        if (!response.ok) throw new Error('Failed to save configuration');

        const result = await response.json();
        showMessage('Configuration saved successfully!', 'success');

        // Also save notifications if provided
        await saveNotifications();

    } catch (error) {
        console.error('Error saving configuration:', error);
        showMessage('Error saving configuration: ' + error.message, 'error');
    }
}

// Save notifications configuration
async function saveNotifications() {
    const gmailAddress = document.getElementById('gmail-address').value.trim();
    const gmailPassword = document.getElementById('gmail-password').value.trim();
    const emailRecipients = document.getElementById('email-recipients').value.trim();
    const discordWebhook = document.getElementById('discord-webhook').value.trim();

    if (!gmailAddress && !discordWebhook) {
        return; // No notification settings to save
    }

    try {
        const config = {
            gmail_address: gmailAddress,
            gmail_app_password: gmailPassword,
            email_recipients: emailRecipients ? emailRecipients.split(',').map(e => e.trim()) : [],
            discord_webhook_url: discordWebhook
        };

        const response = await fetch(`${API_BASE}/notifications`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(config)
        });

        if (!response.ok) throw new Error('Failed to save notification settings');

    } catch (error) {
        console.error('Error saving notifications:', error);
    }
}

// Start monitoring
async function startMonitoring() {
    try {
        const response = await fetch(`${API_BASE}/jobs/start`, {
            method: 'POST'
        });

        if (!response.ok) throw new Error('Failed to start monitoring');

        const result = await response.json();
        showMessage('Monitoring started successfully!', 'success');
        updateStatus();

    } catch (error) {
        console.error('Error starting monitoring:', error);
        showMessage('Error starting monitoring: ' + error.message, 'error');
    }
}

// Stop monitoring
async function stopMonitoring() {
    try {
        const response = await fetch(`${API_BASE}/jobs/stop`, {
            method: 'POST'
        });

        if (!response.ok) throw new Error('Failed to stop monitoring');

        const result = await response.json();
        showMessage('Monitoring stopped successfully!', 'success');
        updateStatus();

    } catch (error) {
        console.error('Error stopping monitoring:', error);
        showMessage('Error stopping monitoring: ' + error.message, 'error');
    }
}

// Update status
async function updateStatus() {
    try {
        const response = await fetch(`${API_BASE}/status`);
        if (!response.ok) throw new Error('Failed to fetch status');

        const status = await response.json();

        // Update connection status
        const connectionStatus = document.getElementById('connection-status');
        connectionStatus.textContent = status.is_connected ? 'Connected' : 'Disconnected';
        connectionStatus.className = `status-value ${status.is_connected ? 'connected' : 'disconnected'}`;

        // Update monitoring status
        const monitoringStatus = document.getElementById('monitoring-status');
        monitoringStatus.textContent = status.is_running ? 'Running' : 'Stopped';
        monitoringStatus.className = `status-value ${status.is_running ? 'running' : 'stopped'}`;

        // Update stats
        document.getElementById('enrollment-attempts').textContent = status.stats.enrollment_attempts;
        document.getElementById('successful-enrollments').textContent = status.stats.successful_enrollments;
        document.getElementById('last-check-time').textContent = status.last_check_time || 'Never';

    } catch (error) {
        console.error('Error updating status:', error);
    }
}

// Start status polling
function startStatusPolling() {
    updateStatus(); // Initial update
    statusInterval = setInterval(updateStatus, 5000); // Update every 5 seconds
}

// Show message
function showMessage(message, type) {
    const messageDiv = document.getElementById('message-display');
    messageDiv.textContent = message;
    messageDiv.className = `message ${type}`;
    messageDiv.classList.remove('hidden');

    setTimeout(() => {
        messageDiv.classList.add('hidden');
    }, 5000);
}

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    if (statusInterval) {
        clearInterval(statusInterval);
    }
});

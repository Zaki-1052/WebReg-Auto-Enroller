// Multi-user WebReg Auto-Enroller Frontend
let clerk;
let sessionToken = null;
let currentUser = null;
let jobs = [];
let currentJobId = null;

// Initialize Clerk
window.addEventListener('load', async () => {
    try {
        clerk = window.Clerk;
        await clerk.load();

        if (clerk.user) {
            await onUserSignedIn(clerk.user);
        } else {
            showAuthButtons();
        }

        setupEventListeners();
    } catch (error) {
        console.error('Error initializing Clerk:', error);
        showError('Failed to initialize authentication');
    }
});

// Setup event listeners
function setupEventListeners() {
    // Sign in/up buttons
    document.getElementById('sign-in-btn')?.addEventListener('click', () => {
        clerk.openSignIn();
    });

    document.getElementById('sign-up-btn')?.addEventListener('click', () => {
        clerk.openSignUp();
    });

    document.getElementById('sign-out-btn')?.addEventListener('click', async () => {
        await clerk.signOut();
        location.reload();
    });

    // Create job button
    document.getElementById('create-job-btn')?.addEventListener('click', () => {
        showJobModal();
    });

    // Close job details
    document.getElementById('close-details-btn')?.addEventListener('click', () => {
        document.getElementById('job-details-section').style.display = 'none';
    });

    // Modal close
    document.querySelector('.close')?.addEventListener('click', () => {
        hideJobModal();
    });

    document.querySelector('.cancel-btn')?.addEventListener('click', () => {
        hideJobModal();
    });

    // Forms
    document.getElementById('job-form')?.addEventListener('submit', handleJobSubmit);
    document.getElementById('notifications-form')?.addEventListener('submit', handleNotificationsSubmit);

    // Add course button
    document.getElementById('add-course-btn')?.addEventListener('click', addCourseField);
}

// Handle user signed in
async function onUserSignedIn(user) {
    currentUser = user;
    sessionToken = await user.getToken();

    // Update UI
    document.getElementById('user-email').textContent = user.primaryEmailAddress.emailAddress;
    document.getElementById('user-info').style.display = 'block';
    document.getElementById('auth-buttons').style.display = 'none';
    document.getElementById('main-content').style.display = 'block';

    // Load user data
    await loadJobs();
    await loadNotifications();
}

// Show auth buttons
function showAuthButtons() {
    document.getElementById('auth-buttons').style.display = 'block';
    document.getElementById('user-info').style.display = 'none';
    document.getElementById('main-content').style.display = 'none';
}

// API helper functions
async function apiRequest(endpoint, options = {}) {
    const headers = {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${sessionToken}`,
        ...options.headers,
    };

    const response = await fetch(endpoint, {
        ...options,
        headers,
    });

    if (!response.ok) {
        throw new Error(`API request failed: ${response.statusText}`);
    }

    return response.json();
}

// Load jobs
async function loadJobs() {
    try {
        showLoading(true);
        const response = await apiRequest('/api/jobs');

        if (response.success) {
            jobs = response.data;
            renderJobs();
        }
    } catch (error) {
        console.error('Error loading jobs:', error);
        showError('Failed to load jobs');
    } finally {
        showLoading(false);
    }
}

// Render jobs list
function renderJobs() {
    const jobsList = document.getElementById('jobs-list');
    const noJobs = document.getElementById('no-jobs');

    if (jobs.length === 0) {
        jobsList.innerHTML = '';
        noJobs.style.display = 'block';
        return;
    }

    noJobs.style.display = 'none';

    jobsList.innerHTML = jobs.map(job => `
        <div class="job-card" data-job-id="${job.id}">
            <div class="job-header">
                <h3>${job.term}</h3>
                <span class="job-status ${job.is_active ? 'active' : 'inactive'}">
                    ${job.is_active ? '● Running' : '○ Stopped'}
                </span>
            </div>
            <div class="job-info">
                <div>Polling: ${job.polling_interval}s</div>
                <div>Threshold: ${job.seat_threshold}</div>
                <div>Mode: ${job.monitoring_mode}</div>
            </div>
            <div class="job-actions">
                <button class="btn btn-small" onclick="viewJob('${job.id}')">View Details</button>
                ${job.is_active
                    ? `<button class="btn btn-small btn-danger" onclick="stopJob('${job.id}')">Stop</button>`
                    : `<button class="btn btn-small btn-primary" onclick="startJob('${job.id}')">Start</button>`
                }
                <button class="btn btn-small btn-danger" onclick="deleteJob('${job.id}')">Delete</button>
            </div>
        </div>
    `).join('');
}

// View job details
async function viewJob(jobId) {
    try {
        showLoading(true);
        const response = await apiRequest(`/api/jobs/${jobId}`);

        if (response.success) {
            renderJobDetails(response.data);
            document.getElementById('job-details-section').style.display = 'block';
        }
    } catch (error) {
        console.error('Error loading job details:', error);
        showError('Failed to load job details');
    } finally {
        showLoading(false);
    }
}

// Render job details
function renderJobDetails(data) {
    const { job, is_running } = data;
    const details = document.getElementById('job-details');

    const coursesHtml = job.courses.map(course => `
        <div class="course-card">
            <h4>${course.department} ${course.course_code}</h4>
            <div class="sections">
                ${course.sections.map(section => `
                    <div class="section">
                        <strong>Lecture:</strong> ${section.lecture}<br>
                        ${section.discussions.length > 0
                            ? `<strong>Discussions:</strong> ${section.discussions.join(', ')}`
                            : ''
                        }
                    </div>
                `).join('')}
            </div>
        </div>
    `).join('');

    const statsHtml = job.stats ? `
        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-value">${job.stats.total_checks}</div>
                <div class="stat-label">Total Checks</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">${job.stats.openings_found}</div>
                <div class="stat-label">Openings Found</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">${job.stats.enrollment_attempts}</div>
                <div class="stat-label">Enrollment Attempts</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">${job.stats.successful_enrollments}</div>
                <div class="stat-label">Successful Enrollments</div>
            </div>
        </div>
    ` : '<p>No statistics available yet.</p>';

    details.innerHTML = `
        <div class="job-info-grid">
            <div><strong>Term:</strong> ${job.term}</div>
            <div><strong>Status:</strong> <span class="${is_running ? 'status-active' : 'status-inactive'}">${is_running ? 'Running' : 'Stopped'}</span></div>
            <div><strong>Polling Interval:</strong> ${job.polling_interval}s</div>
            <div><strong>Seat Threshold:</strong> ${job.seat_threshold}</div>
            <div><strong>Monitoring Mode:</strong> ${job.monitoring_mode}</div>
            <div><strong>Last Check:</strong> ${job.last_check_time || 'Never'}</div>
        </div>

        <h3>Courses</h3>
        ${coursesHtml}

        <h3>Statistics</h3>
        ${statsHtml}
    `;
}

// Start job
async function startJob(jobId) {
    try {
        showLoading(true);
        await apiRequest(`/api/jobs/${jobId}/start`, { method: 'POST' });
        showSuccess('Job started successfully');
        await loadJobs();
    } catch (error) {
        console.error('Error starting job:', error);
        showError('Failed to start job');
    } finally {
        showLoading(false);
    }
}

// Stop job
async function stopJob(jobId) {
    try {
        showLoading(true);
        await apiRequest(`/api/jobs/${jobId}/stop`, { method: 'POST' });
        showSuccess('Job stopped successfully');
        await loadJobs();
    } catch (error) {
        console.error('Error stopping job:', error);
        showError('Failed to stop job');
    } finally {
        showLoading(false);
    }
}

// Delete job
async function deleteJob(jobId) {
    if (!confirm('Are you sure you want to delete this job?')) {
        return;
    }

    try {
        showLoading(true);
        await apiRequest(`/api/jobs/${jobId}`, { method: 'DELETE' });
        showSuccess('Job deleted successfully');
        await loadJobs();
    } catch (error) {
        console.error('Error deleting job:', error);
        showError('Failed to delete job');
    } finally {
        showLoading(false);
    }
}

// Show job modal
function showJobModal() {
    document.getElementById('job-modal').style.display = 'block';
    addCourseField(); // Add initial course field
}

// Hide job modal
function hideJobModal() {
    document.getElementById('job-modal').style.display = 'none';
    document.getElementById('job-form').reset();
    document.getElementById('courses-container').innerHTML = '';
}

// Add course field
let courseCounter = 0;
function addCourseField() {
    const container = document.getElementById('courses-container');
    const courseId = `course-${courseCounter++}`;

    const courseDiv = document.createElement('div');
    courseDiv.className = 'course-field';
    courseDiv.innerHTML = `
        <div class="course-header">
            <h4>Course ${courseCounter}</h4>
            <button type="button" class="btn btn-small btn-danger" onclick="removeCourse(this)">Remove</button>
        </div>
        <div class="form-row">
            <div class="form-group">
                <label>Department</label>
                <input type="text" class="course-dept" placeholder="e.g., CHEM" required>
            </div>
            <div class="form-group">
                <label>Course Code</label>
                <input type="text" class="course-code" placeholder="e.g., 6B" required>
            </div>
        </div>
        <div class="sections-container" data-course-id="${courseId}">
            <!-- Sections will be added here -->
        </div>
        <button type="button" class="btn btn-small" onclick="addSection('${courseId}')">+ Add Section</button>
    `;

    container.appendChild(courseDiv);
    addSection(courseId); // Add initial section
}

// Remove course
function removeCourse(btn) {
    btn.closest('.course-field').remove();
}

// Add section to course
let sectionCounter = 0;
function addSection(courseId) {
    const container = document.querySelector(`[data-course-id="${courseId}"]`);

    const sectionDiv = document.createElement('div');
    sectionDiv.className = 'section-field';
    sectionDiv.innerHTML = `
        <div class="form-row">
            <div class="form-group">
                <label>Lecture Section</label>
                <input type="text" class="section-lecture" placeholder="e.g., A00" required>
            </div>
            <div class="form-group">
                <label>Discussion Sections (comma-separated)</label>
                <input type="text" class="section-discussions" placeholder="e.g., A01, A02">
            </div>
            <button type="button" class="btn btn-small btn-danger" onclick="removeSection(this)">Remove</button>
        </div>
    `;

    container.appendChild(sectionDiv);
}

// Remove section
function removeSection(btn) {
    btn.closest('.section-field').remove();
}

// Handle job form submit
async function handleJobSubmit(e) {
    e.preventDefault();

    const formData = {
        term: document.getElementById('job-term').value,
        cookie: document.getElementById('job-cookie').value,
        polling_interval: parseInt(document.getElementById('job-polling').value),
        seat_threshold: parseInt(document.getElementById('job-threshold').value),
        monitoring_mode: document.getElementById('job-mode').value,
        courses: []
    };

    // Parse courses
    const courseFields = document.querySelectorAll('.course-field');
    courseFields.forEach(courseField => {
        const dept = courseField.querySelector('.course-dept').value;
        const code = courseField.querySelector('.course-code').value;
        const sections = [];

        const sectionFields = courseField.querySelectorAll('.section-field');
        sectionFields.forEach(sectionField => {
            const lecture = sectionField.querySelector('.section-lecture').value;
            const discussions = sectionField.querySelector('.section-discussions').value
                .split(',')
                .map(s => s.trim())
                .filter(s => s);

            sections.push({ lecture, discussions });
        });

        formData.courses.push({
            department: dept,
            course_code: code,
            sections
        });
    });

    try {
        showLoading(true);
        await apiRequest('/api/jobs', {
            method: 'POST',
            body: JSON.stringify(formData)
        });

        showSuccess('Job created successfully');
        hideJobModal();
        await loadJobs();
    } catch (error) {
        console.error('Error creating job:', error);
        showError('Failed to create job');
    } finally {
        showLoading(false);
    }
}

// Load notifications
async function loadNotifications() {
    try {
        const response = await apiRequest('/api/notifications');

        if (response.success) {
            const settings = response.data;
            document.getElementById('gmail-address').value = settings.gmail_address || '';
            document.getElementById('discord-webhook').value = settings.discord_webhook_url || '';

            const recipients = settings.email_recipients || [];
            document.getElementById('email-recipients').value = recipients.join('\n');
        }
    } catch (error) {
        console.error('Error loading notifications:', error);
    }
}

// Handle notifications form submit
async function handleNotificationsSubmit(e) {
    e.preventDefault();

    const formData = {
        gmail_address: document.getElementById('gmail-address').value || null,
        gmail_app_password: document.getElementById('gmail-password').value || null,
        email_recipients: document.getElementById('email-recipients').value
            .split('\n')
            .map(s => s.trim())
            .filter(s => s),
        discord_webhook_url: document.getElementById('discord-webhook').value || null
    };

    try {
        showLoading(true);
        await apiRequest('/api/notifications', {
            method: 'POST',
            body: JSON.stringify(formData)
        });

        showSuccess('Notifications updated successfully');
        document.getElementById('gmail-password').value = ''; // Clear password field
    } catch (error) {
        console.error('Error updating notifications:', error);
        showError('Failed to update notifications');
    } finally {
        showLoading(false);
    }
}

// UI helpers
function showLoading(show) {
    document.getElementById('loading').style.display = show ? 'flex' : 'none';
}

function showError(message) {
    alert(`Error: ${message}`);
}

function showSuccess(message) {
    alert(message);
}

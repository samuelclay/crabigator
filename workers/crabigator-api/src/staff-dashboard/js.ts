// Staff dashboard JavaScript
import { iconGift } from '../dashboard/icons';

export const staffDashboardJs = `
        // ============================================
        // Collapsible Sections
        // ============================================

        // Get collapsed sections from cookie
        function getCollapsedSections() {
            const cookie = document.cookie.split('; ').find(c => c.startsWith('collapsed_sections='));
            if (!cookie) return [];
            try {
                return JSON.parse(decodeURIComponent(cookie.split('=')[1]));
            } catch (e) {
                return [];
            }
        }

        // Save collapsed sections to cookie
        function saveCollapsedSections(sections) {
            const expires = new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toUTCString();
            document.cookie = 'collapsed_sections=' + encodeURIComponent(JSON.stringify(sections)) + '; expires=' + expires + '; path=/';
        }

        // Toggle a section
        function toggleSection(name) {
            const section = document.getElementById('section-' + name);
            if (!section) return;

            const collapsed = getCollapsedSections();
            const isCollapsed = section.classList.contains('collapsed');

            if (isCollapsed) {
                section.classList.remove('collapsed');
                const idx = collapsed.indexOf(name);
                if (idx > -1) collapsed.splice(idx, 1);
            } else {
                section.classList.add('collapsed');
                if (!collapsed.includes(name)) collapsed.push(name);
            }

            saveCollapsedSections(collapsed);
        }

        // Initialize collapsed state from cookie
        function initCollapsedSections() {
            const collapsed = getCollapsedSections();
            collapsed.forEach(name => {
                const section = document.getElementById('section-' + name);
                if (section) section.classList.add('collapsed');
            });
        }

        // Initialize on load
        initCollapsedSections();

        // Chart instances
        let checksChart = null;
        let versionChart = null;

        // Current gift state
        let currentGiftId = null;
        let currentGiftEmail = null;

        // Show toast notification
        function showToast(message, type = 'success') {
            const toast = document.getElementById('toast');
            toast.textContent = message;
            toast.className = 'toast ' + type + ' visible';
            setTimeout(() => {
                toast.classList.remove('visible');
            }, 3000);
        }

        // Format relative time
        function timeAgo(timestamp) {
            const now = Math.floor(Date.now() / 1000);
            const diff = now - timestamp;

            if (diff < 60) return 'just now';
            if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
            if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
            return Math.floor(diff / 86400) + 'd ago';
        }

        // Format timezone offset
        function formatTimezone(offset) {
            if (offset === null) return '-';
            const hours = Math.floor(Math.abs(offset) / 60);
            const mins = Math.abs(offset) % 60;
            const sign = offset <= 0 ? '+' : '-';
            return 'UTC' + sign + hours + (mins ? ':' + mins.toString().padStart(2, '0') : '');
        }

        // Render telemetry table
        function renderTable(rows, stats) {
            const tbody = document.getElementById('telemetry-table');
            document.getElementById('table-count').textContent = rows.length + ' entries';

            // Update section summary (unique machines 24h / all time)
            document.getElementById('sum-machines-24h').textContent = (stats?.devices_24h || 0).toLocaleString();
            document.getElementById('sum-machines-all').textContent = (stats?.total_devices || 0).toLocaleString();
            if (rows.length > 0) {
                document.getElementById('sum-telemetry-last').textContent = timeAgo(rows[0].created_at);
            }

            if (rows.length === 0) {
                tbody.innerHTML = '<tr><td colspan="8" class="loading">No telemetry data yet</td></tr>';
                return;
            }

            tbody.innerHTML = rows.map(row => \`
                <tr>
                    <td class="device-id" title="\${row.device_id}">\${row.device_id.substring(0, 8)}...</td>
                    <td>\${row.machine_name || '-'}</td>
                    <td><span class="os-badge \${row.os || ''}">\${row.os || '-'}</span></td>
                    <td class="os-version">\${row.os_version || '-'}</td>
                    <td class="cli-version">\${row.cli_version || '-'}</td>
                    <td><span class="version-badge">\${row.app_version}</span></td>
                    <td>\${formatTimezone(row.timezone_offset)}</td>
                    <td class="time-ago">\${timeAgo(row.created_at)}</td>
                </tr>
            \`).join('');
        }

        // Update stats
        function updateStats(stats) {
            document.getElementById('total-devices').textContent = stats.total_devices.toLocaleString();
            document.getElementById('checks-24h').textContent = stats.checks_24h.toLocaleString();
            document.getElementById('checks-7d').textContent = stats.checks_7d.toLocaleString();
            document.getElementById('latest-version').textContent = stats.top_version || '-';

            // Update section summary
            document.getElementById('sum-checks-24h').textContent = stats.checks_24h.toLocaleString();
            document.getElementById('sum-checks-all').textContent = stats.total_checks.toLocaleString();
            document.getElementById('sum-devices-all').textContent = stats.total_devices.toLocaleString();
            document.getElementById('sum-version').textContent = stats.top_version || '-';

            if (stats.new_devices_24h > 0) {
                document.getElementById('devices-change').textContent = '+' + stats.new_devices_24h + ' today';
                document.getElementById('devices-change').className = 'stat-change positive';
            }
        }

        // Update checks chart
        function updateChecksChart(data) {
            const ctx = document.getElementById('checks-chart').getContext('2d');

            if (checksChart) {
                checksChart.data.labels = data.labels;
                checksChart.data.datasets[0].data = data.values;
                checksChart.update();
            } else {
                checksChart = new Chart(ctx, {
                    type: 'line',
                    data: {
                        labels: data.labels,
                        datasets: [{
                            label: 'Update Checks',
                            data: data.values,
                            borderColor: '#58a6ff',
                            backgroundColor: 'rgba(88, 166, 255, 0.1)',
                            fill: true,
                            tension: 0.3
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { display: false }
                        },
                        scales: {
                            x: {
                                grid: { color: '#30363d' },
                                ticks: { color: '#8b949e' }
                            },
                            y: {
                                beginAtZero: true,
                                grid: { color: '#30363d' },
                                ticks: { color: '#8b949e' }
                            }
                        }
                    }
                });
            }
        }

        // Update version chart
        function updateVersionChart(data) {
            const ctx = document.getElementById('version-chart').getContext('2d');

            if (versionChart) {
                versionChart.data.labels = data.labels;
                versionChart.data.datasets[0].data = data.values;
                versionChart.update();
            } else {
                versionChart = new Chart(ctx, {
                    type: 'doughnut',
                    data: {
                        labels: data.labels,
                        datasets: [{
                            data: data.values,
                            backgroundColor: [
                                '#58a6ff',
                                '#3fb950',
                                '#d29922',
                                '#f85149',
                                '#a371f7',
                                '#8b949e'
                            ]
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: {
                                position: 'right',
                                labels: { color: '#e6edf3' }
                            }
                        }
                    }
                });
            }
        }

        // Fetch and update data
        async function fetchData() {
            try {
                const response = await fetch('/api/staff/telemetry');
                if (!response.ok) throw new Error('Failed to fetch');
                const data = await response.json();

                renderTable(data.recent, data.stats);
                updateStats(data.stats);
                updateChecksChart(data.checks_by_day);
                updateVersionChart(data.version_distribution);

                document.getElementById('last-update').textContent =
                    '· Updated ' + new Date().toLocaleTimeString();
            } catch (err) {
                console.error('Fetch error:', err);
            }
        }

        // Initial load and auto-refresh every 5 seconds
        fetchData();
        setInterval(fetchData, 5000);

        // ============================================
        // Website Analytics
        // ============================================

        // Analytics chart instances
        let visitorsChart = null;
        let sourcesChart = null;
        let devicesChart = null;
        let browsersChart = null;
        let countriesChart = null;
        let npmChart = null;

        // Format time as "Xm" or "Xs"
        function formatTime(seconds) {
            if (!seconds || seconds === 0) return '-';
            if (seconds < 60) return seconds + 's';
            return Math.floor(seconds / 60) + 'm ' + (seconds % 60) + 's';
        }

        // Update analytics summary stats
        function updateAnalyticsStats(summary) {
            document.getElementById('analytics-visitors-24h').textContent = summary.visitors_24h.toLocaleString();
            document.getElementById('analytics-pageviews-24h').textContent = summary.pageviews_24h.toLocaleString();
            document.getElementById('analytics-avg-time').textContent = formatTime(summary.avg_time_on_page);
            document.getElementById('analytics-bounce-rate').textContent = summary.bounce_rate + '%';
            document.getElementById('analytics-scroll-depth').textContent = summary.avg_scroll_depth + '%';

            // Update section summary
            document.getElementById('sum-visitors-24h').textContent = summary.visitors_24h.toLocaleString();
            document.getElementById('sum-visitors-all').textContent = summary.visitors_all.toLocaleString();
            document.getElementById('sum-signups-all').textContent = summary.signups_all.toLocaleString();
        }

        // Update visitors chart (line)
        function updateVisitorsChart(data) {
            const ctx = document.getElementById('visitors-chart').getContext('2d');
            if (visitorsChart) {
                visitorsChart.data.labels = data.labels;
                visitorsChart.data.datasets[0].data = data.values;
                visitorsChart.update();
            } else {
                visitorsChart = new Chart(ctx, {
                    type: 'line',
                    data: {
                        labels: data.labels,
                        datasets: [{
                            label: 'Visitors',
                            data: data.values,
                            borderColor: '#58a6ff',
                            backgroundColor: 'rgba(88, 166, 255, 0.1)',
                            fill: true,
                            tension: 0.3
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: { legend: { display: false } },
                        scales: {
                            x: { grid: { color: '#30363d' }, ticks: { color: '#8b949e' } },
                            y: { beginAtZero: true, grid: { color: '#30363d' }, ticks: { color: '#8b949e' } }
                        }
                    }
                });
            }
        }

        // Update referrer details table
        function updateReferrerTable(detail) {
            const tbody = document.getElementById('referrer-tbody');
            if (!tbody) return;

            const rows = [];
            for (const [category, domains] of Object.entries(detail)) {
                for (const { domain, visitors } of domains) {
                    rows.push({ category, domain: domain || '(direct)', visitors });
                }
            }

            // Sort by visitors descending
            rows.sort((a, b) => b.visitors - a.visitors);

            if (rows.length === 0) {
                tbody.innerHTML = '<tr><td colspan="3" style="text-align: center; color: #8b949e;">No data</td></tr>';
            } else {
                tbody.innerHTML = rows.map(r =>
                    '<tr><td>' + r.category + '</td><td>' + r.domain + '</td><td style="text-align: right;">' + r.visitors + '</td></tr>'
                ).join('');
            }

            const countEl = document.getElementById('referrer-count');
            if (countEl) countEl.textContent = rows.length + ' domains';
        }

        // Update traffic sources chart (doughnut)
        function updateSourcesChart(data) {
            const ctx = document.getElementById('sources-chart').getContext('2d');
            const colors = ['#58a6ff', '#a371f7', '#3fb950', '#d29922', '#f85149', '#8b949e'];
            if (sourcesChart) {
                sourcesChart.data.labels = data.labels;
                sourcesChart.data.datasets[0].data = data.values;
                sourcesChart.update();
            } else {
                sourcesChart = new Chart(ctx, {
                    type: 'doughnut',
                    data: {
                        labels: data.labels,
                        datasets: [{
                            data: data.values,
                            backgroundColor: colors,
                            borderWidth: 0
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { position: 'right', labels: { color: '#8b949e', boxWidth: 12, padding: 8 } }
                        }
                    }
                });
            }
        }

        // Update devices chart (doughnut)
        function updateDevicesChart(data) {
            const ctx = document.getElementById('devices-chart').getContext('2d');
            const colors = ['#58a6ff', '#a371f7', '#3fb950'];
            if (devicesChart) {
                devicesChart.data.labels = data.labels;
                devicesChart.data.datasets[0].data = data.values;
                devicesChart.update();
            } else {
                devicesChart = new Chart(ctx, {
                    type: 'doughnut',
                    data: {
                        labels: data.labels,
                        datasets: [{ data: data.values, backgroundColor: colors, borderWidth: 0 }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: { legend: { position: 'bottom', labels: { color: '#8b949e', boxWidth: 10, padding: 6, font: { size: 10 } } } }
                    }
                });
            }
        }

        // Update browsers chart (doughnut)
        function updateBrowsersChart(data) {
            const ctx = document.getElementById('browsers-chart').getContext('2d');
            const colors = ['#58a6ff', '#a371f7', '#3fb950', '#d29922', '#f85149', '#8b949e'];
            if (browsersChart) {
                browsersChart.data.labels = data.labels;
                browsersChart.data.datasets[0].data = data.values;
                browsersChart.update();
            } else {
                browsersChart = new Chart(ctx, {
                    type: 'doughnut',
                    data: {
                        labels: data.labels,
                        datasets: [{ data: data.values, backgroundColor: colors, borderWidth: 0 }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: { legend: { position: 'bottom', labels: { color: '#8b949e', boxWidth: 10, padding: 6, font: { size: 10 } } } }
                    }
                });
            }
        }

        // Update countries chart (bar)
        function updateCountriesChart(data) {
            const ctx = document.getElementById('countries-chart').getContext('2d');
            if (countriesChart) {
                countriesChart.data.labels = data.labels;
                countriesChart.data.datasets[0].data = data.values;
                countriesChart.update();
            } else {
                countriesChart = new Chart(ctx, {
                    type: 'bar',
                    data: {
                        labels: data.labels,
                        datasets: [{
                            label: 'Visitors',
                            data: data.values,
                            backgroundColor: '#58a6ff',
                            borderRadius: 4
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        indexAxis: 'y',
                        plugins: { legend: { display: false } },
                        scales: {
                            x: { beginAtZero: true, grid: { color: '#30363d' }, ticks: { color: '#8b949e' } },
                            y: { grid: { display: false }, ticks: { color: '#8b949e' } }
                        }
                    }
                });
            }
        }

        // Update NPM downloads chart (line)
        function updateNpmChart(data) {
            const ctx = document.getElementById('npm-chart').getContext('2d');
            if (npmChart) {
                npmChart.data.labels = data.labels;
                npmChart.data.datasets[0].data = data.values;
                npmChart.update();
            } else {
                npmChart = new Chart(ctx, {
                    type: 'line',
                    data: {
                        labels: data.labels,
                        datasets: [{
                            label: 'Downloads',
                            data: data.values,
                            borderColor: '#3fb950',
                            backgroundColor: 'rgba(63, 185, 80, 0.1)',
                            fill: true,
                            tension: 0.3
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: { legend: { display: false } },
                        scales: {
                            x: { grid: { color: '#30363d' }, ticks: { color: '#8b949e' } },
                            y: { beginAtZero: true, grid: { color: '#30363d' }, ticks: { color: '#8b949e' } }
                        }
                    }
                });
            }
        }

        // Update conversion funnel
        function updateFunnel(funnel) {
            const visit = funnel.visit || 0;
            const install = funnel.install || 0;
            const session = funnel.session || 0;
            const subscriber = funnel.subscriber || 0;

            document.getElementById('funnel-visit-count').textContent = visit.toLocaleString();
            document.getElementById('funnel-install-count').textContent = install.toLocaleString();
            document.getElementById('funnel-session-count').textContent = session.toLocaleString();
            document.getElementById('funnel-subscriber-count').textContent = subscriber.toLocaleString();

            // Calculate and display rates
            if (visit > 0) {
                const installRate = Math.round((install / visit) * 100);
                document.getElementById('funnel-install-rate').textContent = installRate + '%';
                document.getElementById('funnel-install').style.width = Math.max(10, installRate) + '%';
            }
            if (install > 0) {
                const sessionRate = Math.round((session / install) * 100);
                document.getElementById('funnel-session-rate').textContent = sessionRate + '%';
                document.getElementById('funnel-session').style.width = Math.max(5, (session / visit) * 100) + '%';
            }
            if (session > 0) {
                const subRate = Math.round((subscriber / session) * 100);
                document.getElementById('funnel-subscriber-rate').textContent = subRate + '%';
                document.getElementById('funnel-subscriber').style.width = Math.max(3, (subscriber / visit) * 100) + '%';
            }
        }

        // Render email signups table
        function renderSignupsTable(signups) {
            const tbody = document.getElementById('signups-table');
            const countEl = document.getElementById('signups-count');
            countEl.textContent = signups.length + ' signup' + (signups.length !== 1 ? 's' : '');

            if (signups.length === 0) {
                tbody.innerHTML = '<tr><td colspan="5" class="text-muted" style="text-align: center; padding: 24px;">No email signups yet</td></tr>';
                return;
            }

            tbody.innerHTML = signups.map(s => {
                const source = s.utm_source || s.referrer_domain || 'Direct';
                const campaign = s.utm_campaign || '-';
                const promo = s.promo_code || '-';
                return \`
                    <tr>
                        <td>\${s.email}</td>
                        <td>\${source}</td>
                        <td>\${campaign}</td>
                        <td>\${promo}</td>
                        <td class="time-ago">\${timeAgo(s.created_at)}</td>
                    </tr>
                \`;
            }).join('');
        }

        // Fetch and update analytics data
        async function fetchAnalytics() {
            try {
                const response = await fetch('/api/staff/analytics');
                if (!response.ok) throw new Error('Failed to fetch analytics');
                const data = await response.json();

                updateAnalyticsStats(data.summary);
                updateVisitorsChart(data.visitors_by_day);
                updateSourcesChart(data.traffic_sources);
                updateReferrerTable(data.traffic_sources_detail || {});
                updateDevicesChart(data.devices);
                updateBrowsersChart(data.browsers);
                updateCountriesChart(data.countries);
                updateNpmChart(data.npm_downloads);
                updateFunnel(data.funnel);
                renderSignupsTable(data.email_signups);

                document.getElementById('analytics-update').textContent =
                    'Updated ' + new Date().toLocaleTimeString();
            } catch (err) {
                console.error('Fetch analytics error:', err);
            }
        }

        // Initial load and auto-refresh every 30 seconds
        fetchAnalytics();
        setInterval(fetchAnalytics, 30000);

        // ============================================
        // Gift Management
        // ============================================

        // Fetch and render gifts
        async function fetchGifts() {
            try {
                const response = await fetch('/api/staff/gifts');
                if (!response.ok) throw new Error('Failed to fetch gifts');
                const data = await response.json();
                renderGiftsTable(data.gifts || []);
            } catch (err) {
                console.error('Fetch gifts error:', err);
            }
        }

        // Render gifts table
        function renderGiftsTable(gifts) {
            const tbody = document.getElementById('gifts-table');
            const countEl = document.getElementById('gifts-count');
            countEl.textContent = gifts.length + ' gift' + (gifts.length !== 1 ? 's' : '');

            // Update section summary
            const oneDayAgo = Math.floor(Date.now() / 1000) - 86400;
            const gifts24h = gifts.filter(g => g.created_at > oneDayAgo).length;
            const claimedAll = gifts.filter(g => g.status === 'claimed').length;
            document.getElementById('sum-gifts-24h').textContent = gifts24h.toString();
            document.getElementById('sum-gifts-all').textContent = gifts.length.toString();
            document.getElementById('sum-claimed-all').textContent = claimedAll.toString();

            if (gifts.length === 0) {
                tbody.innerHTML = '<tr><td colspan="9" class="empty-state"><span class="empty-icon">${iconGift}</span>No gifts created yet</td></tr>';
                return;
            }

            tbody.innerHTML = gifts.map(gift => {
                const statusClass = gift.status;
                const statusLabel = gift.status.charAt(0).toUpperCase() + gift.status.slice(1);
                const emailDisplay = gift.recipient_email
                    ? '<span class="email-truncate" title="' + gift.recipient_email + '">' + gift.recipient_email + '</span>'
                    : '<span class="text-muted">-</span>';

                return \`
                    <tr>
                        <td><span class="gift-code">\${gift.id}</span></td>
                        <td>\${emailDisplay}</td>
                        <td><span class="duration-badge">\${gift.duration_type}</span></td>
                        <td class="time-ago">\${timeAgo(gift.created_at)}</td>
                        <td><span class="status-badge \${statusClass}">\${statusLabel}</span></td>
                        <td>\${gift.session_count || '-'}</td>
                        <td>\${gift.total_duration_formatted || '-'}</td>
                        <td>\${gift.avg_duration_formatted || '-'}</td>
                        <td class="actions-cell">
                            <button class="btn btn-secondary btn-icon" onclick="copyUrl('\${gift.url}')" title="Copy URL">
                                <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                                    <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 010 1.5h-1.5a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-1.5a.75.75 0 011.5 0v1.5A1.75 1.75 0 019.25 16h-7.5A1.75 1.75 0 010 14.25v-7.5z"/>
                                    <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0114.25 11h-7.5A1.75 1.75 0 015 9.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25h-7.5z"/>
                                </svg>
                            </button>
                            \${gift.recipient_email && !gift.email_sent_at ? \`
                                <button class="btn btn-secondary btn-icon" onclick="sendEmail('\${gift.id}')" title="Send Email">
                                    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                                        <path d="M1.75 2h12.5c.966 0 1.75.784 1.75 1.75v8.5A1.75 1.75 0 0114.25 14H1.75A1.75 1.75 0 010 12.25v-8.5C0 2.784.784 2 1.75 2zM1.5 12.251c0 .138.112.25.25.25h12.5a.25.25 0 00.25-.25V5.809L8.38 9.397a.75.75 0 01-.76 0L1.5 5.809v6.442zm13-8.181v-.32a.25.25 0 00-.25-.25H1.75a.25.25 0 00-.25.25v.32L8 7.88l6.5-3.81z"/>
                                    </svg>
                                </button>
                            \` : ''}
                        </td>
                    </tr>
                \`;
            }).join('');
        }

        // Create gift form handler
        document.getElementById('gift-form').addEventListener('submit', async (e) => {
            e.preventDefault();

            const btn = document.getElementById('create-gift-btn');
            btn.disabled = true;
            btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" class="spin"><circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="30" stroke-linecap="round"/></svg> Creating...';

            const formData = new FormData(e.target);
            const data = {
                duration_type: formData.get('duration_type'),
                recipient_email: formData.get('recipient_email') || undefined
            };

            try {
                const response = await fetch('/api/staff/gifts', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(data)
                });

                if (!response.ok) {
                    const error = await response.json();
                    throw new Error(error.error || 'Failed to create gift');
                }

                const gift = await response.json();

                // Show result
                currentGiftId = gift.id;
                currentGiftEmail = data.recipient_email;

                document.getElementById('gift-url').textContent = gift.url;
                document.getElementById('gift-result').classList.add('visible');

                // Show send email button if email provided
                const sendBtn = document.getElementById('send-email-btn');
                sendBtn.style.display = data.recipient_email ? 'inline-flex' : 'none';

                // Clear form
                document.getElementById('email').value = '';

                // Refresh gifts table
                fetchGifts();

                showToast('Gift created successfully!');
            } catch (err) {
                showToast(err.message, 'error');
            } finally {
                btn.disabled = false;
                btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 4a.5.5 0 0 1 .5.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3A.5.5 0 0 1 8 4z"/></svg> Create Gift';
            }
        });

        // Copy gift URL from result panel
        function copyGiftUrl() {
            const url = document.getElementById('gift-url').textContent;
            copyUrl(url);
            document.getElementById('copy-success').classList.add('visible');
            setTimeout(() => {
                document.getElementById('copy-success').classList.remove('visible');
            }, 2000);
        }

        // Copy any URL
        function copyUrl(url) {
            navigator.clipboard.writeText(url).then(() => {
                showToast('URL copied to clipboard!');
            }).catch(() => {
                showToast('Failed to copy URL', 'error');
            });
        }

        // Send email for current gift
        async function sendGiftEmail() {
            if (!currentGiftId) return;
            await sendEmail(currentGiftId);
            document.getElementById('send-email-btn').style.display = 'none';
        }

        // Send email for a gift
        async function sendEmail(giftId) {
            try {
                const response = await fetch('/api/staff/gifts/' + giftId + '/send-email', {
                    method: 'POST'
                });

                if (!response.ok) {
                    const error = await response.json();
                    throw new Error(error.error || 'Failed to send email');
                }

                const result = await response.json();
                showToast('Email sent to ' + result.sent_to);
                fetchGifts();
            } catch (err) {
                showToast(err.message, 'error');
            }
        }

        // Initial gifts load and refresh
        fetchGifts();
        setInterval(fetchGifts, 5000);
`;

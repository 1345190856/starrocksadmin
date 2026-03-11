import { Component, OnInit, OnDestroy } from '@angular/core';
import { ChatService } from '../../../@core/services/chat.service';
import { AuthService } from '../../../@core/data/auth.service';
import { Subscription } from 'rxjs';

@Component({
    selector: 'ngx-chat-widget',
    styleUrls: ['./chat-widget.component.scss'],
    templateUrl: './chat-widget.component.html',
})
export class ChatWidgetComponent implements OnInit, OnDestroy {
    isOpen = false;
    isSidebarExpanded = false;
    messages: { text: string; isBot: boolean }[] = [];
    newMessage = '';
    loading = false;
    showConfig = false;
    webhookUrl = '';
    currentSessionId = 'default';
    currentMode: 'kn' | 'chatgpt' = 'kn';
    dragPosition = { x: 0, y: 0 };

    // Custom Modules
    customModules: any[] = [];
    showModuleForm = false;
    newModule: any = { name: '', url: '', body: '', is_published: false };
    editingModuleId: number | null = null;

    // Shortcuts
    shortcuts: any[] = [];
    showShortcutForm = false;
    newShortcut: any = { name: '', url: '', body: '', is_published: false };
    editingShortcutId: number | null = null;

    // Mention (@) System
    showMentionDropdown = false;
    mentionSearch = '';
    filteredModules: any[] = [];
    selectedIndex = 0;

    private chatSubscription: Subscription | null = null;
    private authSubscription: Subscription | null = null;
    private triggerSubscription: Subscription | null = null;

    constructor(
        private chatService: ChatService,
        private authService: AuthService
    ) { }

    ngOnInit() {
        this.webhookUrl = this.chatService.getWebhookUrl();
        this.loadAiSettings();

        // Clear/Load messages when user changes (logout/login)
        this.authSubscription = this.authService.currentUser.subscribe(user => {
            if (user) {
                // Check if we should reset position (on every login)
                // We can use a simple flag in session storage to see if we've already set it for this login session
                if (!sessionStorage.getItem('chat_widget_reset_done')) {
                    localStorage.removeItem('chat_widget_pos');
                    sessionStorage.setItem('chat_widget_reset_done', 'true');
                    this.dragPosition = { x: 0, y: 0 };
                } else {
                    this.loadPosition();
                }
                this.messages = this.chatService.getHistory(user.username, this.currentSessionId);
                this.scrollToBottom();
            } else {
                this.messages = [];
                this.currentSessionId = 'default';
                this.isOpen = false; // Hide drawer on logout
                sessionStorage.removeItem('chat_widget_reset_done');
            }
            this.newMessage = '';
            this.loading = false;
            this.stopGeneration();
        });

        // Listen for external triggers
        this.triggerSubscription = this.chatService.trigger$.subscribe(data => {
            this.triggerModule(data.moduleName, data.variables, data.prompt);
        });
    }

    loadAiSettings() {
        this.chatService.getAiSettings().subscribe(settings => {
            this.customModules = settings.filter(s => s.category === 'module');
            this.shortcuts = settings.filter(s => s.category === 'shortcut');
        });
    }

    ngOnDestroy() {
        if (this.authSubscription) this.authSubscription.unsubscribe();
        if (this.triggerSubscription) this.triggerSubscription.unsubscribe();
        this.stopGeneration();
    }

    get isAuthenticated(): boolean {
        return this.authService.isAuthenticated();
    }

    get isAdmin(): boolean {
        return this.authService.currentUserValue?.username === 'admin';
    }

    get currentUser(): string {
        return this.authService.currentUserValue?.username || '';
    }

    canEdit(item: any): boolean {
        return this.currentUser === 'admin' || item.creator === this.currentUser;
    }

    toggleChat() {
        this.isOpen = !this.isOpen;
        if (this.isOpen) {
            this.scrollToBottom();
        }
    }

    toggleSidebar() {
        this.isSidebarExpanded = !this.isSidebarExpanded;
    }

    switchMode(event: Event, direction: 'prev' | 'next') {
        event.stopPropagation();
        this.currentMode = this.currentMode === 'kn' ? 'chatgpt' : 'kn';
    }

    onDragEnded(event: any) {
        const transform = event.source.getFreeDragPosition();
        this.dragPosition = transform;
        localStorage.setItem('chat_widget_pos', JSON.stringify(this.dragPosition));
    }

    private loadPosition() {
        const saved = localStorage.getItem('chat_widget_pos');
        if (saved) {
            try {
                this.dragPosition = JSON.parse(saved);
            } catch (e) {
                this.dragPosition = { x: 0, y: 0 };
            }
        }
    }

    getSessionName(): string {
        if (this.currentSessionId === 'default') return 'AI 智能助手';
        const mod = this.customModules.find(m => String(m.id) === this.currentSessionId);
        if (mod) return `AI ${mod.name}`;
        const shortcut = this.shortcuts.find(s => String(s.id) === this.currentSessionId);
        return shortcut ? `AI ${shortcut.name}` : 'AI 智能助手';
    }

    switchSession(sessionId: string) {
        if (this.currentSessionId === sessionId) return;

        const username = this.authService.currentUserValue?.username;
        if (username) {
            this.currentSessionId = sessionId;
            this.messages = this.chatService.getHistory(username, sessionId);
            this.stopGeneration();
            this.scrollToBottom();
        }
    }

    closeChat() {
        this.isOpen = false;
        this.showConfig = false;
        this.isSidebarExpanded = false;
        this.stopGeneration();
    }

    onInputChange() {
        if (this.newMessage.includes('@')) {
            const parts = this.newMessage.split('@');
            this.mentionSearch = parts[parts.length - 1].toLowerCase();
            // User requested: only show shortcuts in the mention dropdown
            this.filteredModules = this.shortcuts.filter(s =>
                s.name.toLowerCase().includes(this.mentionSearch)
            );
            this.showMentionDropdown = this.filteredModules.length > 0;
            this.selectedIndex = 0; // Reset index on filter change
        } else {
            this.showMentionDropdown = false;
        }
    }

    onKeyDown(event: KeyboardEvent) {
        if (this.showMentionDropdown) {
            if (event.key === 'ArrowDown') {
                event.preventDefault();
                this.selectedIndex = (this.selectedIndex + 1) % this.filteredModules.length;
            } else if (event.key === 'ArrowUp') {
                event.preventDefault();
                this.selectedIndex = (this.selectedIndex - 1 + this.filteredModules.length) % this.filteredModules.length;
            } else if (event.key === 'Enter') {
                event.preventDefault();
                this.selectMention(this.filteredModules[this.selectedIndex]);
            } else if (event.key === 'Escape') {
                this.showMentionDropdown = false;
            }
        }
    }

    selectMention(module: any) {
        const parts = this.newMessage.split('@');
        parts[parts.length - 1] = module.name + ' ';
        this.newMessage = parts.join('@');
        this.showMentionDropdown = false;
    }

    sendMessage() {
        const username = this.authService.currentUserValue?.username;
        if (!this.newMessage.trim() || this.loading || !username) return;

        const userMsg = this.newMessage.trim();

        // Check for @ mention command (Available in ALL sessions)
        const mentionMatch = this.shortcuts.find(s => userMsg.startsWith(`@${s.name}`));
        if (mentionMatch) {
            this.executeModule(mentionMatch);
            this.newMessage = '';
            return;
        }

        // Check if we are in a module or shortcut session
        if (this.currentSessionId !== 'default') {
            const module = this.customModules.find(m => String(m.id) === this.currentSessionId) ||
                this.shortcuts.find(s => String(s.id) === this.currentSessionId);
            if (module) {
                this.messages.push({ text: userMsg, isBot: false });
                this.newMessage = '';
                this.loading = true;
                this.scrollToBottom();

                // Detect which variable to use based on module body or name
                const variables: any = {};
                const bodyStr = typeof module.body === 'string' ? module.body : JSON.stringify(module.body);
                if (bodyStr.includes('${sql}')) {
                    variables.sql = userMsg;
                } else {
                    variables.message = userMsg;
                }

                this.chatSubscription = this.chatService.executeModule(module, username, variables).subscribe({
                    next: (res) => {
                        const responseText = res.output || res.message || res.text ||
                            (typeof res === 'string' ? res : JSON.stringify(res));
                        this.messages.push({ text: responseText, isBot: true });
                        this.loading = false;
                        this.chatService.saveHistory(username, this.messages, this.currentSessionId);
                        this.scrollToBottom();
                    },
                    error: (err) => {
                        console.error('Module execution failed:', err);
                        const errorMsg = err.error?.message || err.message || '网络错误';
                        this.messages.push({ text: 'Error: ' + errorMsg, isBot: true });
                        this.loading = false;
                        this.chatService.saveHistory(username, this.messages, this.currentSessionId);
                        this.scrollToBottom();
                    }
                });
                return;
            }
        }

        this.messages.push({ text: userMsg, isBot: false });
        this.newMessage = '';
        this.loading = true;
        this.scrollToBottom();

        this.chatSubscription = this.chatService.sendMessage(userMsg, username).subscribe({
            next: (res) => {
                const responseText = res.output || res.message || res.text ||
                    (typeof res === 'string' ? res : JSON.stringify(res));
                this.messages.push({ text: responseText, isBot: true });
                this.loading = false;
                this.chatService.saveHistory(username, this.messages, this.currentSessionId);
                this.scrollToBottom();
            },
            error: (err) => {
                console.error('Chat failed:', err);
                const errorMsg = err.error?.message || err.message || '连接助手失败';
                this.messages.push({ text: 'Error: ' + errorMsg, isBot: true });
                this.loading = false;
                this.chatService.saveHistory(username, this.messages, this.currentSessionId);
                this.scrollToBottom();
            }
        });
    }

    stopGeneration() {
        if (this.chatSubscription) {
            this.chatSubscription.unsubscribe();
            this.chatSubscription = null;
        }
        if (this.loading) {
            this.loading = false;
            this.messages.push({ text: '已停止生成。', isBot: true });
            this.scrollToBottom();
        }
    }

    executeModule(module: any) {
        const username = this.authService.currentUserValue?.username;
        if (!username || this.loading) return;

        this.messages.push({ text: `@${module.name}`, isBot: false });
        this.loading = true;
        this.scrollToBottom();

        this.chatSubscription = this.chatService.executeModule(module, username).subscribe({
            next: (res) => {
                // Support both res.output or res.message or the whole res if it's a string
                const responseText = res.output || res.message || res.text ||
                    (typeof res === 'string' ? res : JSON.stringify(res));

                this.messages.push({ text: responseText, isBot: true });
                this.loading = false;
                this.chatService.saveHistory(username, this.messages, this.currentSessionId);
                this.scrollToBottom();
            },
            error: (err) => {
                console.error('Module execution failed:', err);
                const errorMsg = err.error?.message || err.message || '网络错误或服务未响应';
                this.messages.push({ text: '执行失败: ' + errorMsg, isBot: true });
                this.loading = false;
                this.chatService.saveHistory(username, this.messages, this.currentSessionId);
                this.scrollToBottom();
            }
        });
    }

    triggerModule(moduleName: string, variables: { [key: string]: string }, displayPrompt: string) {
        const module = this.customModules.find(m => m.name === moduleName) ||
            this.shortcuts.find(s => s.name === moduleName);
        if (!module) {
            console.warn(`Custom module "${moduleName}" not found.`);
            this.messages.push({ text: displayPrompt, isBot: false });
            this.newMessage = '';
            this.sendMessage(); // Fallback to normal chat
            return;
        }

        const username = this.authService.currentUserValue?.username;
        if (!username || this.loading) return;

        // Automatically switch to the module's session
        const sessionId = String(module.id);
        if (this.currentSessionId !== sessionId) {
            this.currentSessionId = sessionId;
            this.messages = this.chatService.getHistory(username, sessionId);
        }

        this.isOpen = true;
        this.messages.push({ text: displayPrompt, isBot: false });
        this.loading = true;
        this.scrollToBottom();

        this.chatSubscription = this.chatService.executeModule(module, username, variables).subscribe({
            next: (res) => {
                const responseText = res.output || res.message || res.text ||
                    (typeof res === 'string' ? res : JSON.stringify(res));
                this.messages.push({ text: responseText, isBot: true });
                this.loading = false;
                this.chatService.saveHistory(username, this.messages, this.currentSessionId);
                this.scrollToBottom();
            },
            error: (err) => {
                console.error('Module execution failed:', err);
                const errorMsg = err.error?.message || err.message || '连接失败';
                this.messages.push({ text: 'Error: ' + errorMsg, isBot: true });
                this.loading = false;
                this.chatService.saveHistory(username, this.messages, this.currentSessionId);
                this.scrollToBottom();
            }
        });
    }

    addModule() {
        if (!this.newModule.name || !this.newModule.url) return;

        const payload = { ...this.newModule, category: 'module' };
        if (this.editingModuleId !== null) {
            this.chatService.updateAiSetting(this.editingModuleId, payload).subscribe(() => {
                this.loadAiSettings();
                this.editingModuleId = null;
                this.newModule = { name: '', url: '', body: '', is_published: false };
                this.showModuleForm = false;
            });
        } else {
            this.chatService.createAiSetting(payload).subscribe(() => {
                this.loadAiSettings();
                this.newModule = { name: '', url: '', body: '', is_published: false };
                this.showModuleForm = false;
            });
        }
    }

    editModule(module: any) {
        this.editingModuleId = module.id;
        this.newModule = {
            name: module.name,
            url: module.url,
            body: typeof module.body === 'string' ? module.body : JSON.stringify(module.body, null, 2),
            is_published: module.is_published
        };
        this.showModuleForm = true;
    }

    deleteModule(id: number) {
        this.chatService.deleteAiSetting(id).subscribe(() => {
            this.loadAiSettings();
            if (this.editingModuleId === id) {
                this.showModuleForm = false;
                this.editingModuleId = null;
            }
        });
    }

    addShortcut() {
        if (!this.newShortcut.name || !this.newShortcut.url) return;

        const payload = { ...this.newShortcut, category: 'shortcut' };
        if (this.editingShortcutId !== null) {
            this.chatService.updateAiSetting(this.editingShortcutId, payload).subscribe(() => {
                this.loadAiSettings();
                this.editingShortcutId = null;
                this.newShortcut = { name: '', url: '', body: '', is_published: false };
                this.showShortcutForm = false;
            });
        } else {
            this.chatService.createAiSetting(payload).subscribe(() => {
                this.loadAiSettings();
                this.newShortcut = { name: '', url: '', body: '', is_published: false };
                this.showShortcutForm = false;
            });
        }
    }

    editShortcut(shortcut: any) {
        this.editingShortcutId = shortcut.id;
        this.newShortcut = {
            name: shortcut.name,
            url: shortcut.url,
            body: typeof shortcut.body === 'string' ? shortcut.body : JSON.stringify(shortcut.body, null, 2),
            is_published: shortcut.is_published
        };
        this.showShortcutForm = true;
    }

    deleteShortcut(id: number) {
        this.chatService.deleteAiSetting(id).subscribe(() => {
            this.loadAiSettings();
            if (this.editingShortcutId === id) {
                this.showShortcutForm = false;
                this.editingShortcutId = null;
            }
        });
    }

    newSession() {
        const username = this.authService.currentUserValue?.username;
        if (username) {
            this.chatService.resetSessionId(username, this.currentSessionId !== 'default' ? this.currentSessionId : undefined);
            this.messages = [];
            this.newMessage = '';
            this.chatService.saveHistory(username, [], this.currentSessionId);
        }
    }

    saveConfig() {
        this.chatService.setWebhookUrl(this.webhookUrl);
        this.showConfig = false;
    }

    private scrollToBottom() {
        setTimeout(() => {
            const chatBody = document.querySelector('.chat-body');
            if (chatBody) {
                chatBody.scrollTop = chatBody.scrollHeight;
            }
        }, 100);
    }
}

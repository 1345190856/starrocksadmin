import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable, Subject } from 'rxjs';

@Injectable({
    providedIn: 'root'
})
export class ChatService {
    private sessionIdKey = 'chat_session_id';
    private webhookUrlKey = 'chat_webhook_url';
    private defaultWebhook = 'http://localhost:15678/webhook/45cec96e-962d-4e27-ab75-34d25b837032';

    constructor(private http: HttpClient) { }

    getSessionId(username: string, moduleId?: string): string {
        const key = moduleId ? `${this.sessionIdKey}_${username}_${moduleId}` : `${this.sessionIdKey}_${username}`;
        let sessionId = localStorage.getItem(key);
        if (!sessionId) {
            sessionId = this.generateSessionId();
            localStorage.setItem(key, sessionId);
        }
        return sessionId;
    }

    resetSessionId(username: string, moduleId?: string): void {
        const key = moduleId ? `${this.sessionIdKey}_${username}_${moduleId}` : `${this.sessionIdKey}_${username}`;
        const newId = this.generateSessionId();
        localStorage.setItem(key, newId);
        if (!moduleId) {
            this.clearHistory(username);
        }
    }

    private generateSessionId(): string {
        return Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15);
    }

    // History Management
    getHistory(username: string, moduleId: string = 'default'): any[] {
        const historyStr = localStorage.getItem(`chat_history_${username}_${moduleId}`);
        return historyStr ? JSON.parse(historyStr) : [];
    }

    saveHistory(username: string, messages: any[], moduleId: string = 'default'): void {
        localStorage.setItem(`chat_history_${username}_${moduleId}`, JSON.stringify(messages));
    }

    clearHistory(username: string, moduleId: string = 'default'): void {
        localStorage.removeItem(`chat_history_${username}_${moduleId}`);
    }


    // New Database Persisted Settings
    getAiSettings(): Observable<any[]> {
        return this.http.get<any[]>('/api/ai/settings');
    }

    createAiSetting(setting: any): Observable<any> {
        return this.http.post('/api/ai/settings', setting);
    }

    updateAiSetting(id: number, setting: any): Observable<any> {
        return this.http.put(`/api/ai/settings/${id}`, setting);
    }

    deleteAiSetting(id: number): Observable<any> {
        return this.http.delete(`/api/ai/settings/${id}`);
    }

    executeModule(module: any, username: string, variables: { [key: string]: string } = {}): Observable<any> {
        let bodyStr = module.body || '';
        if (typeof bodyStr !== 'string') {
            bodyStr = JSON.stringify(bodyStr);
        }

        const moduleId = module.id ? String(module.id) : undefined;
        const sessionId = this.getSessionId(username, moduleId);

        // Support ${session} variable replacement
        let processedBodyStr = bodyStr.replace(/\$\{session\}/g, sessionId);

        // Replace other custom variables like ${sql}
        Object.keys(variables).forEach(key => {
            const regex = new RegExp(`\\$\\{${key}\\}`, 'g');
            // Sanitize value for JSON if it's being injected into a string field
            const sanitizedValue = variables[key]
                .replace(/\\/g, '\\\\')
                .replace(/"/g, '\\"')
                .replace(/\n/g, '\\n')
                .replace(/\r/g, '\\r')
                .replace(/\t/g, '\\t');
            processedBodyStr = processedBodyStr.replace(regex, sanitizedValue);
        });

        let finalBody;
        if (processedBodyStr.trim()) {
            try {
                finalBody = JSON.parse(processedBodyStr);
            } catch (e) {
                console.error('Failed to parse JSON body, sending as raw string:', e);
                finalBody = processedBodyStr;
            }
        } else {
            finalBody = {};
        }

        return this.http.post((module.url || '').trim(), finalBody);
    }

    getWebhookUrl(): string {
        return localStorage.getItem(this.webhookUrlKey) || this.defaultWebhook;
    }

    setWebhookUrl(url: string): void {
        localStorage.setItem(this.webhookUrlKey, url);
    }

    private triggerSubject = new Subject<{ moduleName: string, variables: any, prompt: string }>();
    trigger$ = this.triggerSubject.asObservable();

    triggerModule(moduleName: string, variables: any, prompt: string) {
        this.triggerSubject.next({ moduleName, variables, prompt });
    }

    sendMessage(message: string, username: string): Observable<any> {
        const url = this.getWebhookUrl();
        const sessionId = this.getSessionId(username);
        return this.http.post(url, { message, sessionId });
    }
}

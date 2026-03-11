import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';

export interface AlertRule {
    id: number;
    name: string;
    region: string;
    tags?: string;
    dataSource: string;
    datasourceId?: number;
    alertType: string;
    subType: string;
    threshold: number;
    starrocksVersion: string;
    channel?: 'tv' | 'ivr';
    templateId?: string;
    ivrTemplate?: string;
    ivrSecret?: string;
    ivrParams?: any;
    receivers: AlertReceiver[];
    enabled: boolean;
    autoKill?: boolean;
    autoKillThresholdMinutes?: number;
    notifyIntervalMinutes?: number;
    createdAt?: string;
    updatedAt?: string;
    channels?: AlertChannel[];
}

export interface AlertChannel {
    type: 'tv' | 'ivr';
    startTime: string; // "00:00"
    endTime: string; // "24:00"
    // TV
    templateId?: string;
    // IVR
    ivrTemplate?: string;
    ivrSecret?: string;
    ivrParams?: any;
    notifyIntervalMinutes?: number;
    // Helper for UI
    ivrParamRows?: { key: string, value: string }[];
}

export interface AlertReceiver {
    name: string;
    email?: string;
    phone?: string;
    role: 'duty' | 'manager';
}

export interface AlertHistory {
    id: number;
    ruleId: number;
    queryId: string;
    startTime?: string;
    user?: string;
    host?: string;
    db?: string;
    department?: string;
    sqlText?: string;
    violationDetail?: string;
    status?: string;
    alertCount?: number;
    lastAlertTime?: string;
    cpuTime?: number;
    memUsage?: number;
    execTime?: number;
    scanRows?: number;
    scanBytes?: number;
    connectionId?: string;
    createdAt: string;
    remark?: string;
    repairPerson?: string;
    ivrMsgId?: string;
}

export interface AlertHistoryResponse {
    items: AlertHistory[];
    total: number;
}

@Injectable({
    providedIn: 'root',
})
export class AlertService {
    private baseUrl = '/api/alert';

    constructor(private http: HttpClient) { }

    getRules(): Observable<AlertRule[]> {
        return this.http.get<AlertRule[]>(`${this.baseUrl}/rules`);
    }

    createRule(rule: Partial<AlertRule>): Observable<AlertRule> {
        return this.http.post<AlertRule>(`${this.baseUrl}/rules`, rule);
    }

    updateRule(id: number, rule: Partial<AlertRule>): Observable<AlertRule> {
        return this.http.put<AlertRule>(`${this.baseUrl}/rules/${id}`, rule);
    }

    deleteRule(id: number): Observable<void> {
        return this.http.delete<void>(`${this.baseUrl}/rules/${id}`);
    }

    testRule(id: number): Observable<any> {
        return this.http.post<any>(`${this.baseUrl}/rules/${id}/test`, {});
    }

    getHistory(
        page: number = 1,
        pageSize: number = 20,
        status?: string,
        cluster?: string,
        user?: string,
        department?: string,
        sortField?: string,
        sortOrder?: string,
        startDate?: string,
        endDate?: string,
        queryId?: string
    ): Observable<AlertHistoryResponse> {
        let params: any = { page: page.toString(), pageSize: pageSize.toString() };
        if (status) params.status = status;
        if (cluster) params.cluster = cluster;
        if (user) params.user = user;
        if (department) params.department = department;
        if (sortField) params.sortField = sortField;
        if (sortOrder) params.sortOrder = sortOrder;
        if (startDate) params.startDate = startDate;
        if (endDate) params.endDate = endDate;
        if (queryId) params.queryId = queryId;

        return this.http.get<AlertHistoryResponse>(`${this.baseUrl}/history`, { params });
    }

    getHistoryDetail(id: number): Observable<AlertHistory> {
        return this.http.get<AlertHistory>(`${this.baseUrl}/history/${id}`);
    }

    killQuery(historyId: number): Observable<any> {
        return this.http.post<any>(`${this.baseUrl}/history/${historyId}/kill`, {});
    }

    whitelistQuery(historyId: number): Observable<any> {
        return this.http.post<any>(`${this.baseUrl}/history/${historyId}/whitelist`, {});
    }

    getHistoryClusters(): Observable<string[]> {
        return this.http.get<string[]>(`${this.baseUrl}/history/clusters`);
    }

    getHistoryDepartments(): Observable<string[]> {
        return this.http.get<string[]>(`${this.baseUrl}/history/departments`);
    }

    updateRemark(id: number, remark: string): Observable<any> {
        return this.http.put<any>(`${this.baseUrl}/history/${id}/remark`, { remark });
    }

    updateRepairPerson(id: number, repairPerson: string): Observable<any> {
        return this.http.put<any>(`${this.baseUrl}/history/${id}/repair_person`, { repairPerson });
    }

    notify(botId: string, message: string, mentions: string[] = []): Observable<any> {
        return this.http.post<any>(`${this.baseUrl}/notify`, { botId, message, mentions });
    }

    getSqlSummary(): Observable<any> {
        return this.http.get<any>(`${this.baseUrl}/summary/sql`);
    }

    getSqlTrend(days: number = 30): Observable<any> {
        return this.http.get<any>(`${this.baseUrl}/summary/sql/trend`, { params: { days: days.toString() } });
    }

    getExternalSummary(payload?: string): Observable<any> {
        if (payload) {
            return this.http.post<any>(`${this.baseUrl}/summary/external`, payload);
        }
        return this.http.get<any>(`${this.baseUrl}/summary/external`);
    }
}

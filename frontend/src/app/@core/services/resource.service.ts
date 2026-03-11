import { Injectable } from '@angular/core';
import { ApiService } from '../data/api.service';
import { Observable } from 'rxjs';

export interface ResourceDataSource {
    id?: number;
    name: string;
    type: 'prometheus' | 'mysql' | 'starrocks';
    url: string;
    username?: string;
    password?: string;
    region?: string;
    fe_mapping?: any;
    connection_timeout?: number;
}

export interface ResourcePanel {
    id: number;
    section: 'department' | 'application' | 'cluster';
    title: string;
    chart_type: 'line' | 'bar' | 'pie' | 'stat' | 'gauge' | 'table';
    promql_query: string;
    config?: any;
    display_order: number;
    data_source_id?: number;
    country?: string;
}

export interface PromQuery {
    query: string;
    start?: number;
    end?: number;
    step?: string;
    data_source_id?: number;
}

@Injectable({
    providedIn: 'root'
})
export class ResourceService {
    constructor(private api: ApiService) { }

    getPanels(): Observable<ResourcePanel[]> {
        return this.api.get('/resource/panels');
    }

    createPanel(data: Partial<ResourcePanel>): Observable<ResourcePanel> {
        return this.api.post('/resource/panels', data);
    }

    updatePanel(id: number, data: Partial<ResourcePanel>): Observable<ResourcePanel> {
        return this.api.put(`/resource/panels/${id}`, data);
    }

    deletePanel(id: number): Observable<any> {
        return this.api.delete(`/resource/panels/${id}`);
    }

    queryPrometheus(query: string, start?: Date, end?: Date, step?: string, dsId?: number): Observable<any> {
        const payload: PromQuery = { query };
        if (start && end) {
            payload.start = start.getTime() / 1000;
            payload.end = end.getTime() / 1000;
            payload.step = step;
        }
        if (dsId) payload.data_source_id = dsId;
        return this.api.post('/resource/query', payload);
    }

    // Data Source APIs
    getDataSources(): Observable<ResourceDataSource[]> {
        return this.api.get('/resource/datasources');
    }

    testDataSource(data: any): Observable<any> {
        return this.api.post('/resource/datasources/test', data);
    }

    createDataSource(data: ResourceDataSource): Observable<ResourceDataSource> {
        return this.api.post('/resource/datasources', data);
    }

    updateDataSource(id: number, data: ResourceDataSource): Observable<ResourceDataSource> {
        return this.api.put(`/resource/datasources/${id}`, data);
    }

    deleteDataSource(id: number): Observable<any> {
        return this.api.delete(`/resource/datasources/${id}`);
    }

    // Deprecated
    getSettings(): Observable<string> {
        return this.api.get('/resource/settings');
    }
    updateSettings(url: string): Observable<any> {
        return this.api.put('/resource/settings', url);
    }
}

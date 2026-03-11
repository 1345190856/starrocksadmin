
import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';
import { ResourceService, ResourceDataSource, ResourcePanel, PromQuery } from '../../@core/services/resource.service';

@Injectable()
export class SharedResourceService extends ResourceService {
    private readonly baseUrl = '/share/resource';

    constructor(private httpClient: HttpClient) {
        super(null as any);
    }

    override getPanels(): Observable<ResourcePanel[]> {
        return this.httpClient.get<ResourcePanel[]>(`${this.baseUrl}/panels`);
    }

    override getDataSources(): Observable<ResourceDataSource[]> {
        return this.httpClient.get<ResourceDataSource[]>(`${this.baseUrl}/datasources`);
    }

    override queryPrometheus(query: string, start?: Date, end?: Date, step?: string, dsId?: number): Observable<any> {
        const payload: PromQuery = { query };
        if (start && end) {
            payload.start = start.getTime() / 1000;
            payload.end = end.getTime() / 1000;
            payload.step = step;
        }
        if (dsId) payload.data_source_id = dsId;
        return this.httpClient.post(`${this.baseUrl}/query`, payload);
    }
}

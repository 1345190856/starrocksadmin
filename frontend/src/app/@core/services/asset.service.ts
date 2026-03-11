import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';

export interface ApiResult<T> {
    code: number;
    message: string;
    data: T;
}

export interface ResourceAsset {
    instance_type: string;
    instance_id: string;
    instance_name: string;
    project_name: string;
    project_ownership: string;
    manual_service: string;
    auto_services: any;
    status: string;
    country: string;
    region: string;
    public_ip: string;
    private_ip: string;
    network_identifier: string;
    cpu: string;
    memory: string;
    storage: string;
    release: string;
    created_at: string;
    remark: string;
    [key: string]: any;
}

export interface ResourceListResponse {
    list: ResourceAsset[];
    total: number;
}

export interface ResourceFilterOptions {
    project_names: string[];
    service_types: string[];
    service_statuses: string[];
    countries: string[];
    regions: string[];
}

@Injectable({
    providedIn: 'root',
})
export class AssetService {
    private apiUrl = '/api/asset';

    constructor(private http: HttpClient) { }

    listResources(params: any): Observable<ApiResult<ResourceListResponse>> {
        return this.http.get<ApiResult<ResourceListResponse>>(`${this.apiUrl}/resources`, { params });
    }

    importResources(items: any[]): Observable<ApiResult<number>> {
        return this.http.post<ApiResult<number>>(`${this.apiUrl}/import`, { items });
    }

    updateResource(data: any): Observable<ApiResult<any>> {
        return this.http.put<ApiResult<any>>(`${this.apiUrl}/resources`, data);
    }

    getFilterOptions(): Observable<ApiResult<ResourceFilterOptions>> {
        return this.http.get<ApiResult<ResourceFilterOptions>>(`${this.apiUrl}/filter-options`);
    }

    deleteResources(privateIps: string[]): Observable<ApiResult<any>> {
        return this.http.post<ApiResult<any>>(`${this.apiUrl}/resources/batch-delete`, { private_ips: privateIps });
    }

    applyResources(data: { ip_list: string[], cookie: string, remarks: string }): Observable<ApiResult<any>> {
        return this.http.post<ApiResult<any>>(`${this.apiUrl}/apply`, data);
    }

    serviceOperation(data: { type: string, service: string, ip: string }): Observable<ApiResult<string>> {
        return this.http.post<ApiResult<string>>(`${this.apiUrl}/service-op`, data);
    }
}

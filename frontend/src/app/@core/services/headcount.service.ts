import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

export interface ApiResult<T> {
    code: number;
    message: string;
    data: T;
}

export interface Employee {
    id: number;
    userId: string;
    user?: string;
    name: string;
    city: string;
    company: string;
    email: string;
    employeeNumber: string;
    phone: string;
    orgFullPath: string;
    position: string;
    join_at?: string;
    leaveAt?: string;
    labor_type_txt?: string;
    status_txt?: string;
    orgName_1?: string;
    orgName_2?: string;
    orgName?: string;
    position_level_mame?: string;
    leaderEmployeeNumber?: string;
    leaderId?: number;
    [key: string]: any;
}

export interface EmployeeListResponse {
    list: Employee[];
    total: number;
}

@Injectable({
    providedIn: 'root',
})
export class HeadcountService {
    private readonly baseUrl = '/api/headcount';

    constructor(private http: HttpClient) { }

    listEmployees(
        page: number = 1,
        pageSize: number = 20,
        query: string = '',
        sortBy: string = '',
        sortOrder: 'asc' | 'desc' = 'asc'
    ): Observable<ApiResult<EmployeeListResponse>> {
        let params = new HttpParams()
            .set('page', page.toString())
            .set('page_size', pageSize.toString());

        if (query) {
            params = params.set('query', query);
        }

        if (sortBy) {
            params = params.set('sort_by', sortBy);
            params = params.set('sort_order', sortOrder);
        }

        return this.http.get<ApiResult<EmployeeListResponse>>(`${this.baseUrl}/employees`, { params });
    }

    syncEmployees(): Observable<ApiResult<string>> {
        return this.http.post<ApiResult<string>>(`${this.baseUrl}/sync`, {});
    }
}

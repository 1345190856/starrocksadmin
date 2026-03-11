
import { NgModule } from '@angular/core';
import { RouterModule, Routes } from '@angular/router';
import { DepartmentResourceComponent } from './resource/department-resource.component';

const routes: Routes = [
    {
        path: 'resource/department',
        component: DepartmentResourceComponent,
    },
];

@NgModule({
    imports: [RouterModule.forChild(routes)],
    exports: [RouterModule],
})
export class ShareRoutingModule {
}

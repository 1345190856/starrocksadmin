import { NgModule } from '@angular/core';
import { Routes, RouterModule } from '@angular/router';
import { ResourceLayoutComponent } from './layout/resource-layout.component';
import { DataSourceListComponent } from './components/datasource-list/datasource-list.component';

const routes: Routes = [{
    path: 'view',
    component: ResourceLayoutComponent,
}, {
    path: 'datasource',
    component: DataSourceListComponent,
}, {
    path: '',
    redirectTo: 'view',
    pathMatch: 'full',
}];

@NgModule({
    imports: [RouterModule.forChild(routes)],
    exports: [RouterModule],
})
export class ResourceRoutingModule { }

import { NgModule } from '@angular/core';
import { NbCardModule, NbIconModule, NbInputModule, NbTreeGridModule, NbButtonModule, NbSelectModule, NbTabsetModule, NbToggleModule, NbTagModule, NbAutocompleteModule, NbPopoverModule } from '@nebular/theme';
import { ScrollingModule } from '@angular/cdk/scrolling';
import { Ng2SmartTableModule } from 'ng2-smart-table';
import { ThemeModule } from '../../@theme/theme.module';
import { AlertRoutingModule } from './alert-routing.module';
import { AlertComponent } from './alert.component';
import { AlertDashboardComponent } from './dashboard/alert-dashboard.component';
import { AlertRulesComponent } from './rules/alert-rules.component';
import { AlertRuleDetailComponent } from './rules/alert-rule-detail.component';
import { FormsModule } from '@angular/forms';
import { NgxEchartsModule } from 'ngx-echarts';

@NgModule({
  imports: [
    ThemeModule,
    NbCardModule,
    NbIconModule,
    NbInputModule,
    NbButtonModule,
    NbSelectModule,
    NbTabsetModule,
    NbToggleModule,
    Ng2SmartTableModule,
    AlertRoutingModule,
    FormsModule,
    NbTagModule,
    NbAutocompleteModule,
    NbPopoverModule,
    ScrollingModule,
    NgxEchartsModule.forChild(),
  ],
  declarations: [
    AlertComponent,
    AlertDashboardComponent,
    AlertRulesComponent,
    AlertRuleDetailComponent,
  ],
})
export class AlertModule { }
